use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{Arch, ArchInt, ArchTrap, ArchTrapContext, ArchUserFloatContext};
use ksync::mutex::check_no_lock;

use crate::{
    cpu::current_cpu,
    sched::utils::{suspend_now, take_waker},
    task::{status::TaskStatus, Task},
    time::gettime::get_time_us,
    trap::handler::user_trap_handler,
};

pub struct UserTaskFuture<F: Future + Send + 'static> {
    pub task: Arc<Task>,
    pub future: F,
}

impl<F: Future + Send + 'static> UserTaskFuture<F> {
    pub fn new(task: Arc<Task>, future: F) -> Self {
        Self { task, future }
    }
}

impl<F: Future + Send + 'static> Future for UserTaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Arch::disable_interrupt();

        let this = unsafe { self.get_unchecked_mut() };
        let task = &this.task;
        let future = &mut this.future;
        let time_in = get_time_us();

        // set task to current cpu, set task status to Running
        current_cpu().set_task(task);

        // execute task future
        let ret = unsafe { Pin::new_unchecked(future).poll(cx) };

        // update vruntime
        let time_out = get_time_us();
        task.sched_entity.update_vruntime(time_out - time_in);
        trace!(
            "task {} yielded, wall_time: {} us, vruntime: {}",
            task.tid(),
            time_out - time_in,
            task.sched_entity.inner().vruntime.0
        );

        // mark current task's freg as should restore
        task.trap_context_mut().freg_mut().yield_task();

        // clear current task
        current_cpu().clear_task();

        // always enable global interrupt before return
        Arch::enable_interrupt();
        ret
    }
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    let mut old_mask = None;
    assert!(check_no_lock());
    loop {
        // kernel -> user
        trace!("[task_main] trap_restore, cx: {:#x?}", task.trap_context());
        Arch::trap_restore(task.trap_context_mut());
        assert!(check_no_lock());
        let mut pcb = task.pcb();
        if let Some(old_mask) = old_mask.take() {
            pcb.pending_sigs.sig_mask = old_mask;
        }
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now(pcb).await,
            _ => drop(pcb),
        }
        assert!(check_no_lock());

        // user -> kernel, enter the handler
        trace!(
            "[task_main] user_trap_handler, cx: {:#x?}",
            task.trap_context()
        );
        assert!(!Arch::is_interrupt_enabled());
        assert!(check_no_lock());
        user_trap_handler(&task).await;
        let pcb = task.pcb();
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now(pcb).await,
            _ => drop(pcb),
        }
        assert!(check_no_lock());

        // check signal before return to user
        trace!("[task_main] check_signal");
        old_mask = task.check_signal();
        assert!(check_no_lock());
        let pcb = task.pcb();
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now(pcb).await,
            _ => drop(pcb),
        }
        assert!(check_no_lock());
    }
    assert!(check_no_lock());
    task.exit_handler().await;
}

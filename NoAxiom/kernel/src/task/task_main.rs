use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{fence, Ordering},
    task::{Context, Poll},
};

use arch::{Arch, ArchInt, ArchTrap, ArchTrapContext, ArchUserFloatContext};
use ksync::mutex::check_no_lock;

use crate::{
    cpu::current_cpu,
    sched::utils::{suspend_now, take_waker},
    task::{status::TaskStatus, Task},
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
        // ===== before executing task future =====
        assert!(Arch::is_interrupt_enabled());
        Arch::disable_interrupt();
        let this = unsafe { self.get_unchecked_mut() };
        let task = &this.task;
        let future = &mut this.future;
        task.time_stat_mut().record_switch_in();
        current_cpu().set_task(task);
        fence(Ordering::AcqRel);
        task.restore_cx_int_en();
        // ===== before executing task future =====

        let ret = unsafe { Pin::new_unchecked(future).poll(cx) };

        // ===== after executing task future =====
        task.record_cx_int_en();
        Arch::disable_interrupt();
        task.time_stat_mut().record_switch_out();
        task.trap_context_mut().freg_mut().yield_task();
        current_cpu().clear_task();
        fence(Ordering::AcqRel);
        Arch::enable_interrupt();
        // ===== after executing task future =====
        ret
    }
}

/// user task main
/// called by [`UserTaskFuture`]
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    assert!(check_no_lock());
    loop {
        // kernel -> user
        trace!("[task_main] trap_restore, cx: {:#x?}", task.trap_context());
        task.time_stat_mut().record_trap_in();
        let cx = task.trap_context_mut();
        Arch::trap_restore(cx); // restore context and return to user mode
        let trap_type = Arch::read_trap_type(Some(cx));
        task.time_stat_mut().record_trap_out();

        // check sigmask and status
        assert!(check_no_lock());
        match task.pcb().status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now().await,
            _ => {}
        }
        assert!(check_no_lock());

        // user -> kernel, enter the handler
        trace!(
            "[task_main] user_trap_handler, cx: {:#x?}",
            task.trap_context()
        );
        assert!(!Arch::is_interrupt_enabled());
        assert!(check_no_lock());
        user_trap_handler(&task, trap_type).await;
        assert!(Arch::is_interrupt_enabled());

        // check status
        match task.pcb().status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now().await,
            _ => {}
        }
        assert!(check_no_lock());

        // check signal before return to user
        trace!("[task_main] check_signal");
        task.check_signal(None).await;
        assert!(check_no_lock());
        match task.pcb().status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => suspend_now().await,
            _ => {}
        }
        assert!(check_no_lock());
    }
    assert!(check_no_lock());
    task.exit_handler().await;
}

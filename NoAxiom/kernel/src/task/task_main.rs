use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{Arch, ArchInt, ArchTrap, ArchTrapContext, ArchUserFloatContext};
use ksync::assert_no_lock;

use crate::{
    cpu::current_cpu,
    mm::memory_set::kernel_space_activate,
    sched::utils::suspend_now,
    task::{status::TaskStatus, Task},
    trap::handler::user_trap_handler,
    with_interrupt_off,
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
        let this = unsafe { self.get_unchecked_mut() };
        let task = &this.task;
        let future = &mut this.future;
        with_interrupt_off!({
            task.time_stat_mut().record_switch_in();
            current_cpu().set_task(task);
            task.memory_activate();
        });
        task.restore_cx_int_en();
        let ret = unsafe { Pin::new_unchecked(future).poll(cx) };
        task.record_cx_int_en();
        // todo: switch to kernel only when memory space drops
        with_interrupt_off!({
            task.time_stat_mut().record_switch_out();
            task.trap_context_mut().freg_mut().yield_task();
            current_cpu().clear_task();
            kernel_space_activate();
        });
        ret
    }
}

/// suspend current task
/// only used in stopped status
pub async fn stop_now(task: &Arc<Task>) {
    assert_no_lock!();
    suspend_now().await;
    task.pcb().set_status(TaskStatus::Normal);
}

/// user task main
/// called by [`UserTaskFuture`]
pub async fn task_main(task: Arc<Task>) {
    task.thread_init().await;
    assert_no_lock!();
    loop {
        // kernel -> user
        trace!("[task_main] trap_restore, cx: {:#x?}", task.trap_context());
        task.time_stat_mut().record_trap_in();
        let cx = task.trap_context_mut();
        Arch::trap_restore(cx); // restore context and return to user mode
        let trap_type = Arch::read_trap_type(Some(cx));
        task.time_stat_mut().record_trap_out();

        // check sigmask and status
        assert_no_lock!();
        let status = task.pcb().status();
        match status {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => stop_now(&task).await,
            _ => {}
        }
        assert_no_lock!();

        // user -> kernel, enter the handler
        trace!(
            "[task_main] user_trap_handler, cx: {:#x?}",
            task.trap_context()
        );
        Arch::disable_interrupt();
        assert_no_lock!();
        user_trap_handler(&task, trap_type).await;
        Arch::enable_interrupt();

        // check status
        let status = task.pcb().status();
        match status {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => stop_now(&task).await,
            _ => {}
        }
        assert_no_lock!();

        // check signal before return to user
        trace!("[task_main] check_signal");
        task.check_signal().await;
        assert_no_lock!();
        let status = task.pcb().status();
        match status {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => stop_now(&task).await,
            _ => {}
        }
        assert_no_lock!();
    }
    assert_no_lock!();
    task.exit_handler().await;
}

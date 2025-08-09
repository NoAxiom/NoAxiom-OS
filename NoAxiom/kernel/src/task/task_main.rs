use alloc::sync::Arc;
use core::{
    future::Future,
    intrinsics::likely,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{Arch, ArchInt, ArchTrapContext, ArchUserFloatContext};
use ksync::assert_no_lock;

use crate::{
    cpu::current_cpu,
    include::process::TaskFlags,
    mm::{memory_set::kernel_space_activate, user_ptr::UserPtr},
    sched::utils::{suspend_now, take_waker},
    task::{status::TaskStatus, Task},
    trap::utrap_handler::user_trap_handler,
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
    task.pcb().set_status(TaskStatus::Normal, task.tif_mut());
}

// check TaskStatus::Terminated and TaskStatus::Stopped
macro_rules! check_status {
    ($task:expr) => {
        assert_no_lock!();
        if let Some(status) = $task.try_get_status() {
            match status {
                TaskStatus::Terminated => break,
                TaskStatus::Stopped => stop_now(&$task).await,
                _ => {}
            }
        }
        assert_no_lock!();
    };
}

impl Task {
    /// init thread only resources
    pub async fn thread_init(self: &Arc<Self>) {
        if let Some(tid) = self.tcb().set_child_tid {
            let ptr = UserPtr::<usize>::new(tid);
            let _ = ptr.write(self.tid()).await.inspect_err(|err| {
                error!(
                    "[kernel] failed to write set_child_tid: {}, tid: {}",
                    err,
                    self.tid()
                )
            });
        }
        self.set_waker(take_waker().await);
    }
    /// try to get task status
    pub fn try_get_status(&self) -> Option<TaskStatus> {
        if likely(!self.tif().contains(TaskFlags::TIF_STATUS_CHANGED)) {
            None
        } else {
            self.tif_mut().remove(TaskFlags::TIF_STATUS_CHANGED);
            Some(self.pcb().status())
        }
    }
}

/// user task main
/// called by [`UserTaskFuture`]
pub async fn task_main(task: Arc<Task>) {
    task.thread_init().await;
    loop {
        // kernel -> user
        check_status!(task);
        let trap_type = task.trap_restore();

        // user -> kernel, enter the handler
        check_status!(task);
        user_trap_handler(&task, trap_type).await;
        Arch::enable_interrupt();

        // check signal before return to user
        check_status!(task);
        task.check_signal().await;
    }
    assert_no_lock!();
    task.exit_handler().await;
}

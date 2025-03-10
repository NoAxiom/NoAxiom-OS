use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use arch::{Arch, ArchInt};

use crate::{
    cpu::current_cpu,
    sched::utils::take_waker,
    task::{status::TaskStatus, Task},
    time::gettime::get_time_us,
    trap::{trap_restore, user_trap_handler},
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
        let time_in = get_time_us();

        // set task to current cpu, set task status to Running
        current_cpu().set_task(task);
        task.set_status(TaskStatus::Running);

        // execute task future
        let ret = unsafe { Pin::new_unchecked(future).poll(cx) };

        // clear current task
        // note that task status will be set in other place
        current_cpu().clear_task();

        // update vruntime
        let time_out = get_time_us();
        task.sched_entity.update_vruntime(time_out - time_in);
        trace!(
            "task {} yielded, wall_time: {} us, vruntime: {}",
            task.tid(),
            time_out - time_in,
            task.sched_entity.inner().vruntime.0
        );

        // normally we set the task to runnable status
        let _ = task.cmp_xchg_status(TaskStatus::Runnable, TaskStatus::Suspended);

        // always enable global interrupt before return
        Arch::enable_global_interrupt();
        ret
    }
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    while !task.is_stopped() {
        // kernel -> user
        trace!("[task_main] trap_restore");
        trap_restore(&task);
        // debug!("cx: {:?}", task.trap_context());
        if task.is_stopped() {
            warn!("task {} is zombie before trap_handler, break", task.tid());
            break;
        }
        // user -> kernel
        trace!("[task_main] user_trap_handler");
        assert!(!Arch::is_interrupt_enabled());
        user_trap_handler(&task).await;
    }
    task.exit_handler().await;
}

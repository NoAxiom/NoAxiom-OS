use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    cpu::current_cpu,
    sched::utils::take_waker,
    task::Task,
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
        let p = current_cpu();
        let time_in = get_time_us();
        p.set_task(&mut this.task);
        trace!("polling task {}", this.task.tid());
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        p.clear_task();
        let time_out = get_time_us();
        this.task.sched_entity.update_vruntime(time_out - time_in);
        trace!(
            "task {} yield, poll time: {} us, vruntime: {}",
            this.task.tid(),
            time_out - time_in,
            this.task.sched_entity.inner().vruntime.0
        );
        ret
    }
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    while !task.is_zombie() {
        // kernel -> user
        trace!("[task_main] trap_restore");
        trap_restore(&task);
        // debug!("cx: {:?}", task.trap_context());
        if task.is_zombie() {
            warn!("task {} is zombie before trap_handler, break", task.tid());
            break;
        }
        // user -> kernel
        trace!("[task_main] user_trap_handler");
        user_trap_handler(&task).await;
    }
    task.exit_handler().await;
}

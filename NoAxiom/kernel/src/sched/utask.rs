//! task future
//! [`UserTaskFuture`] runs in user mode,
//! [`KernelTaskFuture`] runs in supervisor mode.

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::executor;
use crate::{cpu::current_cpu, task::Task};

pub async fn utask_main(_task: Arc<Task>) {
    // TODO: this is for test
    _task.test();

    // task.set_waker(utils::take_waker().await);
    // loop {
    // if task.is_zombie() {
    //     break;
    // }
    // trap_return(&task); // kernel -> user
    // if task.is_zombie() {
    //     break;
    // }
    // user_trap_handler(&task).await; // user -> kernel
    // if task.is_zombie() {
    //     break;
    // }
    // check_interval_timer(&task);
    // let _ = check_signal(&task);
    // }
    // handle_exit(&task);
}

pub struct UserTaskFuture<F: Future + Send + 'static> {
    task: Arc<Task>,
    task_future: F,
}

impl<F: Future + Send + 'static> UserTaskFuture<F> {
    pub fn new(task: Arc<Task>, future: F) -> Self {
        Self {
            task,
            task_future: future,
        }
    }
}

impl<F: Future + Send + 'static> Future for UserTaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let p = current_cpu();
        p.set_task(&mut this.task);
        let ret = unsafe { Pin::new_unchecked(&mut this.task_future).poll(cx) };
        p.clear_task();
        ret
    }
}

// spawn a user task
pub fn spawn_utask(task: Arc<Task>) {
    executor::spawn(UserTaskFuture::new(task.clone(), utask_main(task)));
}

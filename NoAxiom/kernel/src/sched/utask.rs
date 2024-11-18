//! ## task future
//! [`UserTaskFuture`] basically runs in user mode,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::executor;
use crate::{arch::current_cpu, task::Task};

pub async fn utask_main(task: Arc<Task>) {
    // TODO: this is for test
    task.test();

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
    future: F,
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
        p.set_task(&mut this.task);
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        p.clear_task();
        ret
    }
}

/// spawn a user task
pub fn spawn_utask(task: Arc<Task>) {
    executor::spawn_raw(UserTaskFuture::new(task.clone(), utask_main(task)));
}

/// all types of futures should be wrapped in this struct
pub struct TaskFuture<F: Future + Send + 'static> {
    future: F,
    prio: usize,
}

impl<F: Future + Send + 'static> TaskFuture<F> {
    pub fn new(future: F, prio: usize) -> Self {
        Self { future, prio }
    }
}

impl<F: Future + Send + 'static> Future for TaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        ret
    }
}

//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::{
    executor::{self, spawn_raw},
    task_counter::task_count_inc,
};
use crate::{
    cpu::current_cpu,
    task::{spawn_new_process, task_main, Task},
};

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

/// spawn a user task, should be wrapped in async fn
pub fn spawn_task(task: Arc<Task>) {
    executor::spawn_raw(UserTaskFuture::new(task.clone(), task_main(task)));
}

/// schedule: will soon complete resouce alloc and spawn task
pub fn schedule_spawn_new_process(path: usize) {
    task_count_inc();
    spawn_raw(spawn_new_process(path));
}

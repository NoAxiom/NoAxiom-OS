//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::AtomicUsize,
    task::{Context, Poll},
};

use super::executor;
use crate::{
    cpu::current_cpu,
    task::{task_main, Task},
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

/// spawn a user task
pub fn spawn_task(task: Arc<Task>) {
    unsafe {
        super::TASK_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }
    executor::spawn_raw(UserTaskFuture::new(task.clone(), task_main(task)));
}

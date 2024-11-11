//! task future
//! [`UserTaskFuture`] runs in user mode,
//! [`KernelTaskFuture`] runs in supervisor mode.

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{sched::executor, task::task::Task};

pub async fn user_loop(task: Arc<Task>) {
    task.waker(super::take_waker().await);
    loop {
        if task.is_zombie() {
            break;
        }
        trap_return(&task); // kernel -> user
        if task.is_zombie() {
            break;
        }
        user_trap_handler(&task).await; // user -> kernel
        if task.is_zombie() {
            break;
        }
        // check_interval_timer(&task);
        // let _ = check_signal(&task);
    }
    handle_exit(&task);
}

pub struct UserTaskFuture<F: Future + Send + 'static> {
    task: Arc<Task>,
    task_future: F,
    // env: EnvContext,
}

impl<F: Future + Send + 'static> UserTaskFuture<F> {
    pub fn new(task: Arc<Task>, future: F) -> Self {
        Self {
            task,
            task_future: future,
            // env: EnvContext::new(),
        }
    }
}

impl<F: Future + Send + 'static> Future for UserTaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let p = get_current_processor();
        p.push_utask(&mut this.task);
        let ret = unsafe { Pin::new_unchecked(&mut this.task_future).poll(cx) };
        p.pop_utask();
        ret
    }
}

pub struct KernelTaskFuture<F: Future<Output = ()> + Send + 'static> {
    task_future: F,
    // env: EnvContext,
}

impl<F: Future<Output = ()> + Send + 'static> KernelTaskFuture<F> {
    pub fn new(task_future: F) -> Self {
        Self {
            task_future,
            // env: EnvContext::new(),
        }
    }
}

impl<F: Future<Output = ()> + Send + 'static> Future for KernelTaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        // let p = get_current_processor();
        // p.switch_ktask(&mut this.env);
        let ret = unsafe { Pin::new_unchecked(&mut this.task_future).poll(cx) };
        // p.switch_ktask(&mut this.env);
        ret
    }
}

pub fn spawn_utask(task: Arc<Task>) {
    executor::spawn(UserTaskFuture::new(task.clone(), user_loop(task)));
}

pub fn spawn_ktask<F: Future<Output = ()> + Send + 'static>(ktask: F) {
    executor::spawn(KernelTaskFuture::new(ktask));
}

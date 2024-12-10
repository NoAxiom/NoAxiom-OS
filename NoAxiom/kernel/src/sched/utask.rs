//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::{executor::spawn_raw, task_counter::task_count_inc};
use crate::{
    cpu::current_cpu,
    sync::cell::SyncUnsafeCell,
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
        // let current_tid = this.task.tid();
        // debug!("[UserTaskFuture::poll] push_task, tid: {}", current_tid);
        p.set_task(&mut this.task);
        // debug!("[UserTaskFuture::poll] push_task done");
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        p.clear_task();
        // debug!("pop_task, tid: {}", current_tid);
        ret
    }
}

/// schedule: will soon allocate resouces and spawn task
pub fn schedule_spawn_new_process(path: usize) {
    task_count_inc();
    trace!("task_count_inc, counter: {}", unsafe {
        crate::sched::task_counter::TASK_COUNTER.load(core::sync::atomic::Ordering::SeqCst)
    });
    spawn_raw(
        async move {
            let task = Task::new_process(path).await;
            spawn_raw(
                UserTaskFuture::new(task.clone(), task_main(task.clone())),
                task.prio.clone(),
            );
        },
        Arc::new(SyncUnsafeCell::new(0)),
    );
}

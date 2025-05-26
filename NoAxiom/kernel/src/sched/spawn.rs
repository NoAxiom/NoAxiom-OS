//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::future::Future;

use async_task::{Builder, WithInfo};

use super::{runtime::RUNTIME, sched_entity::SchedMetadata, vsched::Runtime};
use crate::task::{
    task_main::{task_main, UserTaskFuture},
    Task,
};

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, task: Option<&Arc<Task>>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let metadata = task
        .map(|task| SchedMetadata::from_task(task))
        .unwrap_or_else(SchedMetadata::default);
    let (runnable, handle) = Builder::new().metadata(metadata).spawn(
        move |_| future,
        WithInfo(move |runnable, info| RUNTIME.schedule(runnable, info)),
    );
    runnable.schedule();
    handle.detach();
}

/// inner spawn: spawn a new user task
pub fn spawn_utask(task: Arc<Task>) {
    warn!("[spawn_utask] new task tid = {}", task.tid());
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        Some(&task),
    );
}

/// spawn a new kernel task
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, None);
}

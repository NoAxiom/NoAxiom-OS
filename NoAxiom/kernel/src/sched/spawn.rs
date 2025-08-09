//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::future::Future;

use super::runtime::RUNTIME;
use crate::{
    sched::vsched::Runtime,
    task::{
        task_main::{task_main, UserTaskFuture},
        Task,
    },
};

/// inner spawn: spawn a new user task
pub fn spawn_utask(task: &Arc<Task>) {
    warn!("[spawn_utask] new task tid = {}", task.tid());
    RUNTIME.spawn(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        Some(task),
    );
}

/// spawn a new kernel task
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn(future, None);
}

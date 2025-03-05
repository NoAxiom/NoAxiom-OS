//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::future::Future;

use super::{
    executor::spawn_raw,
    sched_entity::{SchedEntity, SchedTaskInfo},
    task_counter::task_count_inc,
};
use crate::{
    fs::path::Path,
    task::{
        task_main::{task_main, UserTaskFuture},
        Task,
    },
};

/// inner spawn: spawn a new user task
fn inner_spawn(task: Arc<Task>) {
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        task.sched_entity.ref_clone(),
        Some(SchedTaskInfo {
            task: Arc::downgrade(&task),
        }),
    );
}

/// spawn a new user task
pub fn spawn_utask(task: Arc<Task>) {
    task_count_inc();
    inner_spawn(task);
}

/// spawn a new kernel task
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, SchedEntity::new_bare(), None);
}

/// schedule a kernel_task to spawn a new task
pub fn schedule_spawn_new_process(path: Path) {
    task_count_inc();
    spawn_ktask(async move {
        let task = Task::new_process(path).await;
        inner_spawn(task);
    });
}

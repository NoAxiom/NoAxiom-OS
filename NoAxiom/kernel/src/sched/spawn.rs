//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::future::Future;

use async_task::{Builder, WithInfo};

use super::{
    executor::{TaskScheduleInfo, RUNTIME},
    sched_entity::{SchedEntity, SchedTaskInfo},
};
use crate::{
    fs::path::Path,
    task::{
        task_main::{task_main, UserTaskFuture},
        Task,
    },
};

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(future: F, sched_entity: SchedEntity, task_info: Option<SchedTaskInfo>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) = Builder::new()
        .metadata(TaskScheduleInfo::new(sched_entity, task_info))
        .spawn(
            move |_: &TaskScheduleInfo| future,
            WithInfo(move |runnable, info| RUNTIME.push_with_info(runnable, info)),
        );
    runnable.schedule();
    handle.detach();
}

/// inner spawn: spawn a new user task
pub fn spawn_utask(task: Arc<Task>) {
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        task.sched_entity.ref_clone(),
        Some(SchedTaskInfo {
            task: Arc::downgrade(&task),
        }),
    );
}

/// spawn a new kernel task
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, SchedEntity::new_bare(0), None);
}

/// schedule a kernel_task to spawn a new task
pub fn schedule_spawn_new_process(path: Path) {
    spawn_ktask(async move {
        let task = Task::new_process(path).await;
        spawn_utask(task);
    });
}

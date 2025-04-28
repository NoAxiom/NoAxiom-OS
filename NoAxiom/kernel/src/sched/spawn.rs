//! ## task future
//! [`UserTaskFuture`] represents a user task future,
//! use [`spawn_utask`] to spawn user tasks

use alloc::sync::Arc;
use core::future::Future;

use async_task::{Builder, WithInfo};

use super::{runtime::RUNTIME, sched_entity::SchedEntity, sched_info::SchedInfo, vsched::Runtime};
use crate::{
    cpu::get_hartid,
    task::{
        task_main::{task_main, UserTaskFuture},
        Task,
    },
};

/// Add a raw task into task queue
pub fn spawn_raw<F, R>(
    future: F,
    sched_entity: SchedEntity,
    _hartid: usize,
    task: Option<&Arc<Task>>,
) where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let schedule = WithInfo(move |runnable, info| RUNTIME.schedule(runnable, info));
    let (runnable, handle) = Builder::new()
        .metadata(SchedInfo::new(sched_entity, task))
        .spawn(move |_: &SchedInfo| future, schedule);
    runnable.schedule();
    handle.detach();
}

/// inner spawn: spawn a new user task
pub fn spawn_utask(task: Arc<Task>) {
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        task.sched_entity_ref_cloned(),
        get_hartid(),
        Some(&task),
    );
}

/// spawn a new kernel task
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, SchedEntity::new_bare(0), get_hartid(), None);
}

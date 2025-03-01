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
    executor::spawn_raw,
    sched_entity::{SchedEntity, SchedTaskInfo},
    task_counter::{task_count_dec, task_count_inc},
    utils::take_waker,
};
use crate::{
    cpu::current_cpu,
    fs::path::Path,
    task::{exit::exit_handler, Task},
    time::gettime::get_time_us,
    trap::{trap_restore, user_trap_handler},
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
        let time_in = get_time_us();
        p.set_task(&mut this.task);
        trace!("polling task {}", this.task.tid());
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        p.clear_task();
        let time_out = get_time_us();
        this.task.sched_entity.update_vruntime(time_out - time_in);
        trace!(
            "task {} yield, poll time: {} us, vruntime: {}",
            this.task.tid(),
            time_out - time_in,
            this.task.sched_entity.inner().vruntime.0
        );
        ret
    }
}

/// inner spawn: spawn a new user task
fn inner_spawn(task: Arc<Task>) {
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        task.sched_entity.ref_clone(),
        Some(SchedTaskInfo { task }),
    );
}

/// schedule to allocate resouces and spawn task
pub fn schedule_spawn_new_process(path: Path) {
    task_count_inc();
    spawn_raw(
        async move {
            let task = Task::new_process(path).await;
            inner_spawn(task);
        },
        SchedEntity::new_bare(),
        None,
    );
}

pub fn spawn_utask(task: Arc<Task>) {
    task_count_inc();
    inner_spawn(task);
}

#[allow(unused)]
pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, SchedEntity::new_bare(), None);
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    while !task.is_zombie() {
        // kernel -> user
        trace!("[task_main] trap_restore");
        trap_restore(&task);
        // debug!("cx: {:?}", task.trap_context());
        if task.is_zombie() {
            warn!("task {} is zombie before trap_handler, break", task.tid());
            break;
        }
        // user -> kernel
        trace!("[task_main] user_trap_handler");
        user_trap_handler(&task).await;
    }
    task_count_dec();
    exit_handler(&task);
}

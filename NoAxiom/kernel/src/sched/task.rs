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
    sched_entity::SchedEntity,
    task_counter::{task_count_dec, task_count_inc},
};
use crate::{
    config::fs::INIT_PROC_NAME,
    cpu::current_cpu,
    task::Task,
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
        warn!("polling task {}", this.task.tid());
        let ret = unsafe { Pin::new_unchecked(&mut this.future).poll(cx) };
        p.clear_task();
        let time_out = get_time_us();
        this.task.sched_entity.update_vruntime(time_out - time_in);
        debug!(
            "task {} yield, poll time: {} us, vruntime: {}",
            this.task.tid(),
            time_out - time_in,
            this.task.sched_entity.inner().vruntime.0
        );
        ret
    }
}

/// schedule: will soon allocate resouces and spawn task
pub fn schedule_spawn_new_process() {
    task_count_inc();
    spawn_raw(
        async move {
            let task = Task::new_process(INIT_PROC_NAME).await;
            spawn_raw(
                UserTaskFuture::new(task.clone(), task_main(task.clone())),
                task.sched_entity.ref_clone(),
            );
        },
        SchedEntity::new_bare(),
    );
}

pub fn spawn_utask(task: Arc<Task>) {
    task_count_inc();
    spawn_raw(
        UserTaskFuture::new(task.clone(), task_main(task.clone())),
        task.sched_entity.ref_clone(),
    );
}

pub fn spawn_ktask<F, R>(future: F)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    spawn_raw(future, SchedEntity::new_bare());
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    while !task.is_zombie() {
        // kernel -> user
        trace!("[task_main] trap_restore");
        trap_restore(&task);
        // debug!("cx: {:?}", task.trap_context());
        // todo: is this necessary?
        if task.is_zombie() {
            error!(
                "task {} is set zombie before trap_handler, break",
                task.tid()
            );
            break;
        }
        // user -> kernel
        trace!("[task_main] user_trap_handler");
        user_trap_handler(&task).await;
    }
    task_count_dec();
}

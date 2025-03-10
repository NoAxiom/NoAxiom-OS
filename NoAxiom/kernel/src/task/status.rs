use alloc::sync::Arc;

use atomic_enum::atomic_enum;

use crate::{sched::utils::SuspendFuture, task::Task};

#[atomic_enum]
#[derive(PartialEq)]
pub enum TaskStatus {
    /// a task running on current cpu
    /// note that it's not in scheduler
    Running,

    /// a runnable task saved in scheduler
    Runnable,

    /// a suspended task without being saved in scheduler
    /// instead, its waker would be saved by a specific structure
    /// and it will be woken up later when associated interrupt is triggered
    Suspend,

    /// a zombie task which should execute exit handler
    /// and this task will soon be dropped by parent process
    Zombie,
}

impl Task {
    /// suspend current task
    /// difference with yield_now: it won't wake the task immediately
    pub async fn suspend_now(self: &Arc<Self>) {
        SuspendFuture::new().await;
        assert_ne!(
            self.status(),
            TaskStatus::Suspend,
            "still under suspend status!!!"
        );
    }
}

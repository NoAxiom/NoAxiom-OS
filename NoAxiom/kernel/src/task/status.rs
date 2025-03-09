use alloc::sync::Arc;

use atomic_enum::atomic_enum;

use crate::{sched::utils::SuspendFuture, task::Task};

#[atomic_enum]
#[derive(PartialEq)]
pub enum TaskStatus {
    Runnable,
    Suspend,
    Running,
    Zombie,
}

#[atomic_enum]
#[derive(PartialEq)]
pub enum SuspendReason {
    None,
    WaitSignal,
    WaitChildExit,
}

impl Task {
    /// suspend current task
    /// difference with yield_now: it won't wake the task immediately
    pub async fn suspend_now(self: &Arc<Self>, reason: SuspendReason) {
        self.set_suspend_reason(reason); // don't change order
        self.set_status(TaskStatus::Suspend);
        SuspendFuture::new().await;
        assert_ne!(
            self.status(),
            TaskStatus::Suspend,
            "still under suspend status!!!"
        );
    }
}

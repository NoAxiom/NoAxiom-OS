use atomic_enum::atomic_enum;

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

#[derive(PartialEq, Clone, Copy)]
pub enum TaskStatus {
    /// a task running on current cpu
    /// note that it's not in scheduler
    Running,

    /// a runnable task saved in scheduler
    Runnable,

    /// a suspended task without being saved in scheduler
    /// instead, its waker would be saved by a specific structure
    /// and it will be woken up later when associated interrupt is triggered
    Suspended,

    /// a stopped state indicates that the task will soon enter the
    /// exit_handler and will possibly be set to zombie if the task has a parent
    Stopped,

    /// a zombie task which should execute exit handler
    /// and this task will soon be dropped by its parent process
    /// note that only those who owns parent task can be set to zombie
    Zombie,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TaskStatus {
    /// a task running on current cpu or in the schedule queue
    Runnable,

    /// a suspended task without being saved in scheduler
    /// instead, its waker would be saved by a specific structure
    /// and it will be woken up later when associated interrupt is triggered
    /// todo: impl this
    Suspend,

    /// a suspend status that it won't be interrupted
    /// actually, if you don't use suspend_on function, it will lead to
    /// SuspendNoInt as well
    SuspendNoInt,

    /// a stopped state indicates that the process should suspend in the future
    Stopped,

    /// a terminated state indicates that the task will never return to user
    /// mode, and will soon enter the exit_handler
    /// and will set to zombie if the task has a parent
    Terminated,

    /// a zombie task which should execute exit handler
    /// and this task will soon be dropped by its parent process
    /// note that only those who owns parent task can be set to zombie
    Zombie,
}

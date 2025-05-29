#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TaskStatus {
    /// normal status indicates that the task is not marked as any other special
    /// status and it is running or waiting to be scheduled
    ///
    /// - when the task is in scheduler, it indicates that the task is runnable
    /// - when the task is in execution, it indicates that the task is running
    /// - when the task is in reactor, it indicates that the task is suspended
    Normal,

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

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Normal
    }
}

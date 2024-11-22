use alloc::sync::Arc;
use core::task::Waker;

use crate::task::Task;

/// system call tracer for a task
pub struct Syscall {
    task: Arc<Task>,
    waker: Option<Waker>,
}

impl Syscall {
    pub fn new(task: &Arc<Task>) -> Self {
        Self {
            task: task.clone(),
            waker: None,
        }
    }
    pub fn task(&self) -> Arc<Task> {
        self.task.clone()
    }
    pub fn waker(&self) -> Option<Waker> {
        self.waker.clone()
    }
    pub async fn syscall(&self, id: usize, args: [usize; 6]) -> isize {
        info!("syscall id: {}, args: {:?}", id, args);
        -1
    }
}

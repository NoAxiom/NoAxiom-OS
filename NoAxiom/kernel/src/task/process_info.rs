use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};

use super::Task;

/// process control block
/// acturally it contains task's ordinary control info
pub struct PCB {
    /// children tasks, holds lifetime
    pub children: Vec<Arc<Task>>,

    /// zombie children
    pub zombie_children: Vec<Arc<Task>>,

    /// parent task, weak ptr
    pub parent: Option<Weak<Task>>,

    /// wait request
    pub wait_req: bool,

    // /// exit code
    // pub exit_code: i32,
}

impl PCB {
    pub fn find_child(&self, tid: usize) -> Option<&Arc<Task>> {
        if let Some(task) = self.children.iter().find(|task| task.tid() == tid) {
            Some(task)
        } else if let Some(task) = self.zombie_children.iter().find(|task| task.tid() == tid) {
            Some(task)
        } else {
            None
        }
    }
}

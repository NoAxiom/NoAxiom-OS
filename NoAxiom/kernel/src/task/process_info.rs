use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};

use super::Task;

/// process resources info
pub struct ProcessInfo {
    /// children tasks, holds lifetime
    pub children: Vec<Arc<Task>>,

    /// zombie children
    pub zombie_children: Vec<Arc<Task>>,

    /// parent task, weak ptr
    pub parent: Option<Weak<Task>>,

    /// wait request
    pub wait_req: bool,
}

impl ProcessInfo {
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

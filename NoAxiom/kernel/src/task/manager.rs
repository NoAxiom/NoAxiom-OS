use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};

use ksync::{mutex::SpinLock, Lazy};

use super::{
    taskid::{PGID, TID},
    Task,
};
use crate::config::task::INIT_PROCESS_ID;

pub struct TaskManager(pub SpinLock<BTreeMap<TID, Weak<Task>>>);
impl TaskManager {
    pub const fn new() -> Self {
        TaskManager(SpinLock::new(BTreeMap::new()))
    }

    /// insert a task
    pub fn insert(&self, task: &Arc<Task>) {
        self.0.lock().insert(task.tid(), Arc::downgrade(task));
    }

    /// remove a task by tid
    pub fn remove(&self, tid: TID) {
        self.0.lock().remove(&tid);
    }

    /// try to get a task by tid
    pub fn get(&self, tid: TID) -> Option<Arc<Task>> {
        self.0.lock().get(&tid).and_then(|weak| weak.upgrade())
    }

    /// get INIT_PROC task
    pub fn get_init_proc(&self) -> Arc<Task> {
        self.get(INIT_PROCESS_ID).unwrap()
    }
}

pub struct ProcessGroupManager(SpinLock<BTreeMap<PGID, Vec<Weak<Task>>>>);
impl ProcessGroupManager {
    pub const fn new() -> Self {
        Self(SpinLock::new(BTreeMap::new()))
    }

    /// insert a process into a process group
    pub fn insert_process(&self, pgid: PGID, proc: Weak<Task>) {
        let mut inner = self.0.lock();
        match inner.get(&pgid).cloned() {
            Some(mut vec) => {
                vec.push(proc);
                inner.insert(pgid, vec);
            }
            None => {
                let mut vec = Vec::new();
                vec.push(proc);
                inner.insert(pgid, vec);
            }
        }
    }

    /// get all process in one process group
    pub fn get(&self, pgid: PGID) -> Vec<Weak<Task>> {
        self.0.lock().get(&pgid).cloned().unwrap()
    }
}

pub struct ThreadGroup(pub BTreeMap<TID, Weak<Task>>);
impl ThreadGroup {
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }
    pub fn insert(&mut self, task: &Arc<Task>) {
        self.0.insert(task.tid(), Arc::downgrade(&task));
    }
    pub fn remove(&mut self, taskid: usize) {
        self.0.remove(&taskid);
    }
}

pub static TASK_MANAGER: Lazy<TaskManager> = Lazy::new(TaskManager::new);
pub static PROCESS_GROUP_MANAGER: Lazy<ProcessGroupManager> = Lazy::new(ProcessGroupManager::new);

pub fn add_new_process(new_process: &Arc<Task>) {
    new_process.thread_group().insert(new_process);
    TASK_MANAGER.insert(&new_process);
    PROCESS_GROUP_MANAGER.insert_process(new_process.pgid(), Arc::downgrade(new_process));
}

impl Task {
    pub fn delete_children(&self) {
        if self.is_group_leader() {
            // process resources clean up
            let mut pcb = self.pcb();
            // clear all children
            if !pcb.children.is_empty() {
                for child in pcb.children.iter() {
                    // let init_proc take over the child
                    let init_proc = TASK_MANAGER.get_init_proc();
                    child.pcb().parent = Some(Arc::downgrade(&init_proc));
                    init_proc.pcb().children.push(child.clone());
                }
                pcb.children.clear();
            }
            trace!("[delete_children] task {} delete all children", self.tid());
        }
    }
}

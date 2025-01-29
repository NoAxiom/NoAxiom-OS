use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};

use ksync::{mutex::SpinLock, Lazy};

use super::{
    taskid::{PGID, PID, TGID, TID},
    Task,
};
use crate::config::task::INIT_PROCESS_ID;

pub struct TaskManager(SpinLock<BTreeMap<TID, Weak<Task>>>);

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
    pub fn init_task(&self) -> Option<Arc<Task>> {
        self.get(INIT_PROCESS_ID)
    }
}

pub struct ProcessGroupManager(pub SpinLock<BTreeMap<PGID, Vec<PID>>>);

impl ProcessGroupManager {
    pub const fn new() -> Self {
        Self(SpinLock::new(BTreeMap::new()))
    }

    /// insert a process into a process group
    pub fn insert_process(&self, pgid: PGID, pid: PID) {
        let mut inner = self.0.lock();
        match inner.get(&pgid).cloned() {
            Some(mut vec) => {
                vec.push(pid);
                inner.insert(pgid, vec);
            }
            None => {
                let mut vec: Vec<PID> = Vec::new();
                vec.push(pid);
                inner.insert(pgid, vec);
            }
        }
    }

    /// get all process in one process group
    pub fn get_pid_by_pgid(&self, pgid: PGID) -> Vec<PID> {
        self.0.lock().get(&pgid).cloned().unwrap()
    }

    /// modify the process group of a process
    pub fn modify_pgid(&self, pid: PID, new_pgid: PGID, old_pgid: PGID) {
        let mut inner = self.0.lock();
        let old_group_vec = inner.get_mut(&old_pgid).unwrap();
        old_group_vec.retain(|&x| x != pid);
        let new_group_vec = inner.get_mut(&new_pgid);
        if let Some(new_group_vec) = new_group_vec {
            new_group_vec.push(pid);
        } else {
            let new_group: Vec<PID> = vec![pid];
            inner.insert(new_pgid, new_group);
        }
    }
}

pub static TASK_MANAGER: Lazy<TaskManager> = Lazy::new(TaskManager::new);
pub static PROCESS_GROUP_MANAGER: Lazy<ProcessGroupManager> = Lazy::new(ProcessGroupManager::new);

pub fn add_new_process(new_process: &Arc<Task>) {
    new_process.thread_group.lock().insert(new_process);
    TASK_MANAGER.insert(&new_process);
    PROCESS_GROUP_MANAGER.insert_process(new_process.pgid(), new_process.tid());
}

pub struct ThreadGroup(pub BTreeMap<TGID, Weak<Task>>);

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
    pub fn total_num(&self) -> usize {
        self.0.len()
    }
}

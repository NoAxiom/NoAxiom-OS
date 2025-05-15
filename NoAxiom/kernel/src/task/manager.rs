use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};

use ksync::{mutex::SpinLock, Lazy};

use super::{
    taskid::{PGID, PID, TID},
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

#[derive(Clone)]
pub struct ProcessTracer {
    pid: PID,
    proc: Weak<Task>,
}
impl ProcessTracer {
    pub fn pid(&self) -> PID {
        self.pid
    }
    pub fn task(&self) -> Arc<Task> {
        self.proc.upgrade().unwrap()
    }
    pub fn new(task: &Arc<Task>) -> Self {
        let pid = task.tgid();
        Self {
            pid,
            proc: Arc::downgrade(task),
        }
    }
}

pub struct ProcessGroupManager(BTreeMap<PGID, Vec<ProcessTracer>>);
impl ProcessGroupManager {
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// insert a task with provided pgid
    pub fn insert(&mut self, task: &Arc<Task>) {
        let pgid = task.get_pgid();
        self.0
            .entry(pgid)
            .or_insert(Vec::new())
            .push(ProcessTracer::new(task));
    }

    /// remove a process from its process group
    pub fn remove(&mut self, task: &Arc<Task>) {
        let pgid = task.get_pgid();
        let pid = task.tgid();
        self.0
            .get_mut(&pgid)
            .unwrap()
            .retain(|task| task.pid() != pid);
    }

    /// modify the process's pgid into new_pgid
    pub fn modify_pgid(&mut self, task: &Arc<Task>, new_pgid: PGID) {
        self.remove(task);
        task.set_pgid(new_pgid);
        self.insert(task);
    }

    /// create a new process group from existed task
    pub fn create_new_group_by(&mut self, task: &Arc<Task>) {
        self.modify_pgid(task, task.tid());
    }

    /// get all process in one process group
    pub fn get_group(&self, pgid: PGID) -> Option<Vec<ProcessTracer>> {
        self.0.get(&pgid).cloned()
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

pub static TASK_MANAGER: Lazy<TaskManager> = Lazy::new(|| TaskManager::new());
pub static PROCESS_GROUP_MANAGER: Lazy<SpinLock<ProcessGroupManager>> =
    Lazy::new(|| SpinLock::new(ProcessGroupManager::new()));

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

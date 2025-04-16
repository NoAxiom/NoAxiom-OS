use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};

use include::errno::Errno;
use ksync::{mutex::SpinLock, Lazy};

use super::{
    taskid::{PGID, TID},
    Task,
};
use crate::{config::task::INIT_PROCESS_ID, syscall::SyscallResult};

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

    /// insert a new process group with forced setting pgid
    pub fn insert_new_group(&self, group_leader: &Arc<Task>) {
        let pgid = group_leader.tid();
        group_leader.set_pgid(pgid);
        let mut group = Vec::new();
        group.push(Arc::downgrade(group_leader));
        self.0.lock().insert(pgid, group);
    }

    pub fn insert_process(&self, pgid: PGID, process: &Arc<Task>) {
        if !process.is_group_leader() {
            error!(
                "[pg_manager] process {} is not a group leader",
                process.tid()
            );
            return;
        }
        process.set_pgid(pgid);
        let mut inner = self.0.lock();
        let vec = inner.get_mut(&pgid).unwrap();
        vec.push(Arc::downgrade(process));
    }

    /// get all process in one process group
    pub fn get_group(&self, pgid: PGID) -> Option<Vec<Weak<Task>>> {
        self.0.lock().get(&pgid).cloned()
    }

    /// remove a process from its process group
    pub fn remove(&self, process: &Arc<Task>) {
        self.0
            .lock()
            .get_mut(&process.pgid())
            .unwrap()
            .retain(|task| {
                task.upgrade()
                    .map_or(false, |task| Arc::ptr_eq(process, &task))
            })
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
    assert!(new_process.is_group_leader());
    new_process.thread_group().insert(new_process);
    TASK_MANAGER.insert(&new_process);
    PROCESS_GROUP_MANAGER.insert_new_group(&new_process);
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

use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};

use bitflags::Flags;

use super::{exit::ExitReason, status::TaskStatus, Task};
use crate::{
    include::process::{robust_list::RobustList, TaskFlags},
    signal::{sig_pending::SigManager, sig_set::SigSet, sig_stack::SigAltStack},
};

/// task control block inner
/// it is protected by a spinlock to assure its atomicity
/// so there's no need to use any lock in this struct
#[repr(align(64))]
pub struct PCB {
    // task status
    pub status: TaskStatus,    // task status
    pub exit_code: ExitReason, // exit code

    // paternity
    // assertion: only when the task is group leader, it can have children
    pub children: Vec<Arc<Task>>,   // children tasks
    pub parent: Option<Weak<Task>>, // parent task, weak ptr

    // signal structs
    pub signals: SigManager,            // pending signals
    pub sig_stack: Option<SigAltStack>, // signal alternate stack

    // futex & robust list
    pub robust_list: RobustList,
}

impl Default for PCB {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            parent: None,
            status: TaskStatus::Normal,
            exit_code: ExitReason::default(),
            signals: SigManager::new(),
            sig_stack: None,
            robust_list: RobustList::default(),
        }
    }
}

impl PCB {
    // task status
    #[inline(always)]
    pub fn status(&self) -> TaskStatus {
        self.status
    }
    #[inline(always)]
    pub fn set_status(&mut self, status: TaskStatus, tif: &mut TaskFlags) {
        tif.insert(TaskFlags::TIF_STATUS_CHANGED);
        self.status = status;
    }

    // exit code
    pub fn exit_code(&self) -> ExitReason {
        self.exit_code
    }
    pub fn set_exit_code(&mut self, exit_code: ExitReason) {
        self.exit_code = exit_code;
    }

    /// set wake signal
    pub fn set_wake_signal(&mut self, set: SigSet) {
        self.signals.should_wake = set;
    }
    pub fn clear_wake_signal(&mut self) {
        self.signals.should_wake.clear();
    }

    /// find zombie children
    pub fn pop_one_zombie_child(&mut self) -> Option<Arc<Task>> {
        let mut res = None;
        for i in 0..self.children.len() {
            if self.children[i].pcb().status() == TaskStatus::Zombie {
                res = Some(self.children.remove(i));
                break;
            }
        }
        res
    }
}

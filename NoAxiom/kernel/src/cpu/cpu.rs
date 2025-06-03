use alloc::sync::Arc;

use arch::{Arch, ArchAsm};
use ksync::cell::SyncUnsafeCell;

use crate::{config::cpu::CPU_NUM, task::Task};

#[inline(always)]
pub fn get_hartid() -> usize {
    Arch::get_hartid()
}

#[repr(align(64))]
pub struct Cpu {
    /// pointer of current task on this hart
    pub task: Option<Arc<Task>>,
    pub ktrap_depth: usize,
}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            task: None,
            ktrap_depth: 0,
        }
    }
    pub fn add_trap_depth(&mut self) {
        self.ktrap_depth += 1;
        if self.ktrap_depth > 2 {
            error!("[percpu] ktrap_depth > 2, depth: {}", self.ktrap_depth);
        }
    }
    pub fn sub_trap_depth(&mut self) {
        if self.ktrap_depth > 0 {
            self.ktrap_depth -= 1;
        } else {
            panic!("[percpu] ktrap_depth underflow");
        }
    }
    pub fn trap_depth(&self) -> usize {
        self.ktrap_depth
    }
    pub fn set_task(&mut self, task: &Arc<Task>) {
        self.task = Some(task.clone());
    }
    pub fn clear_task(&mut self) {
        self.task = None;
    }
    pub fn current_task(&self) -> &Option<Arc<Task>> {
        &self.task
    }
}

const DEFAULT_CPU: SyncUnsafeCell<Cpu> = SyncUnsafeCell::new(Cpu::new());
pub static CPUS: [SyncUnsafeCell<Cpu>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

pub fn current_cpu() -> &'static mut Cpu {
    CPUS[get_hartid()].as_ref_mut()
}

pub fn current_task() -> Option<&'static Arc<Task>> {
    current_cpu().current_task().as_ref()
}

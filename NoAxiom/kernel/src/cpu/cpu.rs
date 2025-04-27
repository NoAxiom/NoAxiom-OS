use alloc::sync::Arc;

use arch::{Arch, ArchAsm};
use ksync::cell::SyncUnsafeCell;

use crate::{
    config::cpu::CPU_NUM, mm::memory_set::kernel_space_activate, task::Task,
    time::time_slice::set_next_trigger,
};

#[inline(always)]
pub fn get_hartid() -> usize {
    Arch::get_hartid()
}

#[repr(align(64))]
pub struct Cpu {
    /// pointer of current task on this hart
    pub task: Option<Arc<Task>>,
}

impl Cpu {
    pub const fn new() -> Self {
        Self { task: None }
    }
    pub fn set_task(&mut self, task: &Arc<Task>) {
        set_next_trigger();
        self.task = Some(task.clone());
        task.memory_activate();
    }
    pub fn clear_task(&mut self) {
        kernel_space_activate();
        self.task = None;
    }
    pub fn current_task(&self) -> &Arc<Task> {
        &self.task.as_ref().unwrap()
    }
}

const DEFAULT_CPU: SyncUnsafeCell<Cpu> = SyncUnsafeCell::new(Cpu::new());
pub static CPUS: [SyncUnsafeCell<Cpu>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

pub fn current_cpu() -> &'static mut Cpu {
    CPUS[get_hartid()].as_ref_mut()
}

pub fn current_task() -> &'static Arc<Task> {
    current_cpu().current_task()
}

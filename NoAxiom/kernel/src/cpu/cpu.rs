use alloc::sync::Arc;

use arch::{Arch, ArchHart};
use ksync::cell::SyncUnsafeCell;

use crate::{
    config::arch::CPU_NUM, mm::memory_set::kernel_space_activate, task::Task,
    time::timer::set_next_trigger,
};

#[inline(always)]
pub fn get_hartid() -> usize {
    Arch::get_hartid()
}

pub struct Cpu {
    /// pointer of current task on this hart
    pub task: Option<Arc<Task>>,
}

impl Cpu {
    pub const fn new() -> Self {
        Self { task: None }
    }
    pub fn set_task(&mut self, task: &mut Arc<Task>) {
        set_next_trigger();
        self.task = Some(task.clone());
        unsafe { task.memory_activate() };
    }
    pub fn clear_task(&mut self) {
        unsafe { kernel_space_activate() };
        self.task = None;
    }
}

const DEFAULT_CPU: SyncUnsafeCell<Cpu> = SyncUnsafeCell::new(Cpu::new());
pub static mut CPUS: [SyncUnsafeCell<Cpu>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

pub fn current_cpu() -> &'static mut Cpu {
    unsafe { &mut CPUS[get_hartid()] }.get_mut()
}

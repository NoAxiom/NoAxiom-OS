use alloc::sync::Arc;

use crate::{config::CPU_NUM, sync::cell::SyncUnsafeCell, task::Task};

#[inline(always)]
pub fn hartid() -> usize {
    let hartid: usize;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
    hartid
}

pub struct Cpu {
    pub task: Option<Arc<Task>>,
}

impl Cpu {
    pub const fn new() -> Self {
        Self { task: None }
    }

    pub fn set_raw_task(&mut self, task: Arc<Task>) {
        self.task = Some(task);
    }
    pub fn clear_raw_task(&mut self) {
        self.task = None;
    }

    // TODO: mm
    pub fn set_task(&mut self, task: &mut Arc<Task>) {
        self.set_raw_task(task.clone());
    }
    pub fn clear_task(&mut self) {
        self.clear_raw_task();
    }
}

const DEFAULT_CPU: SyncUnsafeCell<Cpu> = SyncUnsafeCell::new(Cpu::new());
pub static mut CPUS: [SyncUnsafeCell<Cpu>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

pub fn current_cpu() -> &'static mut Cpu {
    unsafe { &mut CPUS[hartid()] }.get_mut()
}

// TODO: add mm
// pub fn init(hart_id: usize) {
//     // debug!("start to init hart {}...", hart_id);
//     let hart = get_current_processor();
//     hart.id = hart_id;
//     let sp = get_sp();
//     println!("[kernel][hart{}] set_hart_stack: sp {:#x}", hart.id, sp);
//     // hart.set_stack((sp & !(PAGE_SIZE - 1)) + PAGE_SIZE);
//     unsafe {
//         sstatus::set_fs(FS::Initial);
//     }
// }

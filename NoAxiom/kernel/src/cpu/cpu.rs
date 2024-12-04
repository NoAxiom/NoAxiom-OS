use alloc::sync::Arc;

use crate::{config::arch::CPU_NUM, sync::cell::SyncUnsafeCell, task::Task};

#[inline(always)]
pub fn hartid() -> usize {
    let hartid: usize;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
    hartid
}

pub struct Cpu {
    /// pointer of current task on this hart
    pub task: Option<Arc<Task>>,

    /// the time recorded at current task is lauched
    pub time: usize,
}

impl Cpu {
    pub const fn new() -> Self {
        Self {
            task: None,
            time: 0,
        }
    }
    fn set_raw_task(&mut self, task: Arc<Task>) {
        unsafe {
            task.memory_activate();
        }
        self.task = Some(task);
    }
    pub fn set_task(&mut self, task: &mut Arc<Task>) {
        self.set_raw_task(task.clone());
    }
    pub fn set_time(&mut self, time: usize) {
        self.time = time;
    }

    fn clear_raw_task(&mut self) {
        self.task = None;
    }
    pub fn clear_task(&mut self) {
        self.clear_raw_task();
    }

    pub fn token(&self) -> usize {
        let task = self.task.clone();
        task.unwrap().token()
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

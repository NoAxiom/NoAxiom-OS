use config::cpu::CPU_NUM;
use strum::FromRepr;

pub const SCHED_OTHER: isize = 0;
pub const SCHED_FIFO: isize = 1;
pub const SCHED_RR: isize = 2;
pub const SCHED_BATCH: isize = 3;
pub const SCHED_IDLE: isize = 5;
pub const SCHED_DEADLINE: isize = 6;

type SchedPolicy = isize;

#[repr(C)]
pub struct SchedParam {
    sched_priority: isize,
}
impl SchedParam {
    pub fn new() -> Self {
        Self { sched_priority: 0 }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(&self.sched_priority as *const isize as *const u8, 8) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(&mut self.sched_priority as *mut isize as *mut u8, 8)
        }
    }
    pub fn set_priority(&mut self, priority: isize) {
        self.sched_priority = priority;
    }
    pub fn get_priority(&self) -> isize {
        self.sched_priority
    }
}

const CPU_MASK_SIZE: usize = 1024 / (8 * core::mem::size_of::<u8>());
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuMask {
    mask: [u8; CPU_MASK_SIZE],
}
impl CpuMask {
    pub fn new_bare() -> Self {
        Self {
            mask: [0; CPU_MASK_SIZE],
        }
    }
    pub fn set(&mut self, cpu: usize) {
        let index = cpu / CPU_MASK_SIZE;
        let offset = cpu % CPU_MASK_SIZE;
        self.mask[index] |= 1 << offset;
    }
    pub fn get(&self, cpu: usize) -> bool {
        let index = cpu / CPU_MASK_SIZE;
        let offset = cpu % CPU_MASK_SIZE;
        self.mask[index] & (1 << offset) != 0
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.mask
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.mask
    }
}
impl Default for CpuMask {
    fn default() -> Self {
        let mut new_mask = Self::new_bare();
        for i in 0..CPU_NUM {
            new_mask.set(i);
        }
        new_mask
    }
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(FromRepr)]
pub enum PriorityWhich {
    Process = 0, // WHO is a process ID.
    Pgrp = 1,    // WHO is a process group ID.
    User = 2,    // WHO is a user ID.
}

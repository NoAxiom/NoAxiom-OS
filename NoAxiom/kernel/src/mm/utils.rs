use super::address::VirtAddr;
use crate::{config::mm::PAGE_SIZE, cpu::current_cpu};

/// check if the slice is well-allocated
/// actually you SHOULDN'T call this function
/// because any unallcated memory access will cause a page fault
/// and will be handled by the kernel_trap_handler => memory_validate
pub fn validate_slice(buf: *mut u8, len: usize) {
    warn!("[validate_slice] SHOULDN'T call this function");
    let buf_start: usize = VirtAddr::from(buf as usize).floor().into();
    let buf_end: usize = VirtAddr::from(buf as usize + len).ceil().into();
    let task = current_cpu().task.as_ref().unwrap();
    for va in (buf_start..buf_end).step_by(PAGE_SIZE) {
        task.memory_validate(va);
    }
}

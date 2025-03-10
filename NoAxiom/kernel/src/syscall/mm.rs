use super::SyscallResult;
use crate::{
    config::mm::PAGE_SIZE,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    mm::address::VirtAddr,
    syscall::Syscall,
    utils::align_up,
};

impl Syscall<'_> {
    pub fn sys_brk(&self, brk: usize) -> SyscallResult {
        trace!("[sys_brk] brk: {:#x}", brk);
        if brk == 0 {
            Ok(self.task.memory_set().lock().user_brk as isize)
        } else {
            self.task.grow_brk(brk)
        }
    }

    pub fn sys_mmap(
        &self,
        addr: usize,
        length: usize,
        prot: usize,
        flags: usize,
        fd: isize,
        offset: usize,
    ) -> SyscallResult {
        trace!("[sys_mmap] start");
        let length = align_up(length, PAGE_SIZE);
        let prot = MmapProts::from_bits(prot).unwrap();
        let flags = MmapFlags::from_bits(flags).unwrap();
        if addr % PAGE_SIZE != 0 || length == 0 {
            return Err(Errno::EINVAL);
        }
        self.task
            .mmap(addr, length, prot, flags, fd, offset)
            .map(|addr| addr as isize)
    }

    pub fn sys_munmap(&self, start: usize, length: usize) -> SyscallResult {
        self.task
            .memory_set()
            .lock()
            .mmap_manager
            .remove(VirtAddr(start), length);
        Ok(0)
    }
}

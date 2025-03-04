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
        let task = self.task;
        let former_addr = task.update_brk(0);
        if brk == 0 {
            return Ok(former_addr as isize);
        }
        let grow_size: isize = (brk - former_addr) as isize;
        Ok(self.task.update_brk(grow_size) as isize)
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
        debug!("[sys_mmap] start");
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

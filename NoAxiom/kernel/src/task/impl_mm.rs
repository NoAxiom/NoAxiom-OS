use alloc::sync::Arc;

use super::Task;
use crate::{
    config::mm::USER_HEAP_LIMIT,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    mm::address::VirtAddr,
    return_errno,
    syscall::{SysResult, SyscallResult},
};

impl Task {
    pub fn grow_brk(self: &Arc<Self>, new_brk: usize) -> SyscallResult {
        let mut memory_set = self.memory_set().lock();
        let grow_size = new_brk - memory_set.brk.end;
        info!(
            "[grow_brk] start: {:#x}, old_brk: {:#x}, new_brk: {:#x}, grow_size: {:#x}",
            memory_set.brk.start, memory_set.brk.end, new_brk, grow_size
        );
        if grow_size > 0 {
            trace!("[grow_brk] expanded");
            let growed_addr: usize = memory_set.brk.end + grow_size as usize;
            let limit = memory_set.brk.start + USER_HEAP_LIMIT;
            if growed_addr > limit {
                return_errno!(Errno::ENOMEM);
            }
            memory_set.brk.end = growed_addr;
        } else {
            trace!("[grow_brk] shrinked");
            if new_brk < memory_set.brk.start {
                return_errno!(Errno::EINVAL);
            }
            memory_set.brk.end = new_brk;
        }
        memory_set.brk_grow(VirtAddr(new_brk).ceil());
        Ok(memory_set.brk.end as isize)
    }

    pub fn mmap(
        &self,
        addr: usize,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        fd: isize,
        offset: usize,
    ) -> SysResult<usize> {
        // check file validity, and fetch file from fd_table
        let fd_table = self.fd_table();
        if !flags.contains(MmapFlags::MAP_ANONYMOUS)
            && (fd as usize >= fd_table.table.len() || fd_table.table[fd as usize].is_none())
        {
            return Err(Errno::EBADF);
        }

        // get start_va
        let mut memory_set = self.memory_set().lock();
        let mut start_va = VirtAddr::from(addr);
        if addr == 0 {
            start_va = memory_set.mmap_manager.mmap_top;
        }

        // if contains fix flag, should remove the existing mapping
        if flags.contains(MmapFlags::MAP_FIXED) {
            start_va = VirtAddr::from(addr);
            memory_set.mmap_manager.remove(start_va, length);
        }

        // get target file
        let file = if flags.contains(MmapFlags::MAP_ANONYMOUS) {
            None
        } else {
            fd_table.get(fd as usize)
        };

        // push mmap range (without immediate mapping)
        debug!(
            "[mmap] start_va: {:#x}, length: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {:#x}",
            start_va.0, length, prot, flags, fd, offset
        );
        memory_set
            .mmap_manager
            .insert(start_va, length, prot, flags, offset, file);
        Ok(start_va.0)
    }
}

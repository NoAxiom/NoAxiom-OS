use alloc::sync::Arc;

use super::Task;
use crate::{
    config::mm::USER_HEAP_SIZE,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    mm::address::VirtAddr,
    return_errno,
    syscall::{SysResult, SyscallResult},
};

impl Task {
    /// [`crate::mm::memory_set::MemorySet::lazy_alloc_brk`]
    pub fn grow_brk(self: &Arc<Self>, new_brk: usize) -> SyscallResult {
        let mut ms = self.memory_set().lock();
        info!(
            "[grow_brk] {} start: {:#x}, old_brk: {:#x}, new_brk: {:#x}",
            if new_brk > ms.brk.end {
                "grow"
            } else {
                "shrink"
            },
            ms.brk.start,
            ms.brk.end,
            new_brk,
        );

        if new_brk > ms.brk.end {
            if new_brk > ms.brk.start + USER_HEAP_SIZE {
                return_errno!(Errno::ENOMEM);
            }
            ms.brk.end = new_brk;
        } else {
            if new_brk < ms.brk.start {
                return_errno!(Errno::EINVAL);
            }
            ms.brk.end = new_brk;
        }
        ms.brk_grow(VirtAddr(new_brk).ceil());
        let brk_end = ms.brk.end;

        // for debug
        // let range = ms.brk.area.vpn_range.clone();
        // drop(ms);
        // for vpn in range {
        //     let ptr = vpn.as_va_usize() as *const u8;
        //     let value = unsafe { ptr.read_volatile() };
        //     debug!("[brk] ptr: {:#x}, value: {:#x}", ptr as usize, value);
        // }

        Ok(brk_end as isize)
    }

    /// [`crate::mm::memory_set::lazy_alloc_mmap`]
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
            "[mmap] addr: {:#x}, start_va: {:#x}, length: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {:#x}",
            addr, start_va.0, length, prot, flags, fd, offset
        );
        memory_set
            .mmap_manager
            .insert(start_va, length, prot, flags, offset, file);
        Ok(start_va.0)
    }
}

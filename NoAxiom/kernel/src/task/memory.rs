use alloc::{string::String, sync::Arc};

use config::mm::PAGE_SIZE;

use super::Task;
use crate::{
    config::mm::USER_HEAP_SIZE,
    include::{
        fs::FileFlags,
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
        ms.brk_grow(VirtAddr::from(new_brk).ceil())?;
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
        mut length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        fd: isize,
        offset: usize,
    ) -> SysResult<usize> {
        // check file validity, and fetch file from fd_table
        const LENGTH_OFFSET: usize = 4096 * 32;
        let qwq = if flags.contains(MmapFlags::MAP_STACK) {
            length += LENGTH_OFFSET;
            true
        } else {
            false
        };
        let fd_table = self.fd_table();
        if !flags.contains(MmapFlags::MAP_ANONYMOUS)
            && (fd as usize >= fd_table.table.len() || fd_table.table[fd as usize].is_none())
        {
            return_errno!(Errno::EBADF);
        }

        // get start_va
        let mut memory_set = self.memory_set().lock();
        let mut start_va = VirtAddr::from(addr);
        if addr == 0 {
            start_va = memory_set.mmap_manager.mmap_top;
        }

        // if contains fix flag, should remove the existing mapping
        if flags.contains(MmapFlags::MAP_FIXED_NOREPLACE) {
            if memory_set
                .mmap_manager
                .mmap_map
                .iter()
                .any(|(vpn, _)| vpn.as_va_usize() == start_va.raw())
            {
                return_errno!(Errno::EEXIST);
            }
        }
        if flags.contains(MmapFlags::MAP_FIXED) {
            start_va = VirtAddr::from(addr);
            memory_set.mmap_manager.remove(start_va, length)?;
        }

        // get target file
        let file = if flags.contains(MmapFlags::MAP_ANONYMOUS) {
            None
        } else {
            Some(fd_table.get(fd as usize).ok_or(Errno::EBADF)?)
        };

        if let Some(file) = file.as_ref() {
            let file_flags = file.flags();
            debug!(
                "[mmap] file: {}, flags: {:?}, prot: {:?}",
                file.name(),
                file_flags,
                prot
            );
            if prot.contains(MmapProts::PROT_READ) && !file.meta().readable() {
                return_errno!(Errno::EACCES);
            }
            if file_flags.contains(FileFlags::O_WRONLY) {
                return_errno!(Errno::EACCES);
            }
            if flags.contains(MmapFlags::MAP_SHARED)
                && prot.contains(MmapProts::PROT_WRITE)
                && !file_flags.contains(FileFlags::O_RDWR)
            {
                return_errno!(Errno::EACCES);
            }
            if prot.contains(MmapProts::PROT_WRITE) && file_flags.contains(FileFlags::O_APPEND) {
                return_errno!(Errno::EACCES);
            }
        }

        // push mmap range (without immediate mapping)
        debug!(
            "[mmap] addr: {:#x}, start_va: {:#x}, length: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {:#x}",
            addr, start_va.raw(), length, prot, flags, fd, offset
        );
        let mut res = memory_set
            .mmap_manager
            .insert(start_va, length, prot, flags, offset, file)?;
        if qwq {
            res += LENGTH_OFFSET
        }
        Ok(res)
    }

    pub fn get_maps_string(&self) -> String {
        // fixme: this impl is incorrect
        let mut res = String::new();
        let memory_set = self.memory_set().lock();
        if memory_set.mmap_manager.mmap_map.is_empty() {
            return res;
        }
        for (vpn, mmap_page) in memory_set.mmap_manager.mmap_map.iter() {
            let va = vpn.as_va_usize();
            let perm = mmap_page.get_maps_string();
            res.push_str(&format!(
                "{:08x}-{:08x} {:4} {:08x} {:5} {:5} {}\n",
                va,
                va + PAGE_SIZE,
                perm,
                mmap_page.offset,
                "00:00",
                0,
                ""
            ));
        }
        res
    }
}

use arch::{Arch, ArchMemory, ArchPageTableEntry, MappingFlags};
use memory::address::VpnRange;

use super::SyscallResult;
use crate::{
    config::mm::PAGE_SIZE,
    include::{
        ipc::{IpcCtlCmd, SHM_MAX, SHM_MIN},
        mm::{Madv, MmapFlags, MmapProts},
        result::Errno,
    },
    mm::{address::VirtAddr, page_table::PageTable, permission::MapPermission, shm::SHM_MANAGER},
    return_errno,
    syscall::Syscall,
    utils::align_ceil,
};

impl Syscall<'_> {
    pub fn sys_brk(&self, brk: usize) -> SyscallResult {
        trace!("[sys_brk] brk: {:#x}", brk);
        if brk == 0 {
            let res = self.task.memory_set().lock().brk.end;
            debug!("[sys_brk] get brk, brk.end = {:#x}", res);
            Ok(res as isize)
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
        let length = align_ceil(length, PAGE_SIZE);
        let prot = MmapProts::from_bits_truncate(prot);
        let flags = MmapFlags::from_bits_truncate(flags);
        info!(
            "[sys_mmap] addr: {:#x}, length: {:#x}, prot: {:?}, flags: {:?}, fd: {}, offset: {:#x}",
            addr, length, prot, flags, fd, offset
        );
        if addr % PAGE_SIZE != 0
            || length == 0
            || (addr == 0 && flags.contains(MmapFlags::MAP_FIXED))
            || !(flags.contains(MmapFlags::MAP_SHARED) || flags.contains(MmapFlags::MAP_PRIVATE))
        {
            return_errno!(Errno::EINVAL);
        }
        let res = self.task.mmap(addr, length, prot, flags, fd, offset)?;
        Ok(res as isize)
    }

    pub fn sys_munmap(&self, start: usize, length: usize) -> SyscallResult {
        warn!("sys_munmap: start: {:#x}, length: {:#x}", start, length);
        let start_va = VirtAddr::from(start);
        let end_va = VirtAddr::from(start + length);
        let mut ms = self.task.memory_set().lock();
        ms.mmap_manager.remove(VirtAddr::from(start), length)?;
        let vpn_range = VpnRange::new_from_va(start_va, end_va)?;
        for vpn in vpn_range {
            ms.page_table().unmap(vpn);
        }
        Arch::tlb_flush();
        Ok(0)
    }

    pub fn sys_mprotect(&self, addr: usize, length: usize, prot: usize) -> SyscallResult {
        let root_ppn = Arch::current_root_ppn();
        let page_table = PageTable::from_ppn(root_ppn);

        let map_flags_raw = ((prot & 0b111) << 1) + (1 << 4);
        let map_perm = MapPermission::from_bits(map_flags_raw).unwrap();
        let mapping_flags: MappingFlags = map_perm.into();

        let start_va = VirtAddr::from(addr);
        let end_va = VirtAddr::from(addr + length);
        let vpn_range = VpnRange::new_from_va(start_va, end_va)?;
        if !start_va.is_aligned() {
            return_errno!(Errno::EINVAL);
        }

        info!(
            "[sys_mprotect] range: {:?}, map_perm: {:?}, mapping_flags: {:?}",
            vpn_range, map_perm, mapping_flags
        );

        for vpn in vpn_range {
            if let Some(pte) = page_table.find_pte(vpn) {
                let old_flags = pte.flags();
                let flags = pte.flags().union(mapping_flags);
                pte.set_flags(flags);
                debug!(
                    "[sys_mprotect] set flags in page table, vpn: {:#x}, flags: {:?} => {:?}, pte_raw: {:?}",
                    vpn.raw(), old_flags, flags, pte.raw_flag()
                );
            } else {
                let task = self.task;
                let mut memory_set = task.memory_set().lock();
                let page_table = unsafe { &mut *memory_set.page_table.get() };
                let mmap_start = memory_set.mmap_manager.mmap_start;
                let mmap_top = memory_set.mmap_manager.mmap_top;
                let mmap_prots = MmapProts::from_bits(prot).unwrap();
                let va = VirtAddr::from(vpn);
                if va >= mmap_start && va < mmap_top {
                    memory_set
                        .mmap_manager
                        .mprotect(vpn, mmap_prots, page_table)?;
                } else {
                    return Err(Errno::EINVAL);
                }
            }
        }
        Arch::tlb_flush();
        Ok(0)
    }

    /// create a shared memory segment with the given key and size
    /// and return the segment identifier.
    pub fn sys_shmget(&self, key: usize, size: usize, shmflg: usize) -> SyscallResult {
        warn!(
            "[shmget] key: {:#x}, size: {:#x}, shmflg: {:#x}",
            key, size, shmflg
        );
        if size < SHM_MIN || size > SHM_MAX {
            return Err(Errno::EINVAL);
        }
        let size = align_ceil(size, PAGE_SIZE);
        let shmid = SHM_MANAGER.lock().get(key, size, shmflg)?;
        Ok(shmid as isize)
    }

    /// remove the shared memory segment with the given key
    pub fn sys_shmctl(&self, key: usize, cmd: usize, _buf: *const u8) -> SyscallResult {
        warn!("[shmctl] remove shm key: {:#x}, cmd: {:#x}", key, cmd);
        let cmd = IpcCtlCmd::from_repr(cmd).ok_or(Errno::EINVAL)?;
        if cmd == IpcCtlCmd::IPC_RMID {
            SHM_MANAGER.lock().remove(key);
        } else {
            error!("[shmctl] cmd {:?} is not IPC_RMID", cmd);
            return Err(Errno::ENOSYS);
        }
        Ok(0)
    }

    /// attach the shared memory segment with the given key
    pub fn sys_shmat(&self, key: usize, addr: usize, _shmflg: usize) -> SyscallResult {
        warn!("[shmat] attach shm key {:?} shm address {:#x}", key, addr);
        let task = self.task;
        let mut memory_set = task.memory_set().lock();
        let addr = if addr == 0 {
            memory_set.shm.shm_top
        } else {
            addr
        };
        memory_set.attach_shm(key, addr.into())?;
        drop(memory_set);
        Ok(addr as isize)
    }

    /// detach the shared memory segment with the given address
    pub fn sys_shmdt(&self, address: usize) -> SyscallResult {
        warn!("[shmdt] detach shm address {:#x}", address);
        let task = self.task;
        let mut memory_set = task.memory_set().lock();
        let nattch = memory_set.detach_shm(address.into());
        drop(memory_set);
        Ok(nattch as isize)
    }

    /// Unimplemente advise
    /// todo: finish this!!
    pub fn sys_madvise(addr: usize, len: usize, advice: i32) -> SyscallResult {
        let madvise = Madv::from_repr(advice).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_madvise]: addr: {:#x}, len: {:#x}, advice: {:?}",
            addr, len, madvise
        );
        if addr % PAGE_SIZE != 0 {
            return Err(Errno::EINVAL);
        }
        Ok(0)
    }
}

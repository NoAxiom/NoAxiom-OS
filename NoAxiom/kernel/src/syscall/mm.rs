use arch::{Arch, ArchMemory, ArchPageTableEntry, MappingFlags};
use memory::address::VpnRange;

use super::SyscallResult;
use crate::{
    config::mm::PAGE_SIZE,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    mm::{address::VirtAddr, page_table::PageTable, permission::MapPermission},
    return_errno,
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
        let length = align_up(length, PAGE_SIZE);
        let prot = MmapProts::from_bits(prot).unwrap();
        let flags = MmapFlags::from_bits(flags).unwrap();
        if addr % PAGE_SIZE != 0 || length == 0 {
            return Err(Errno::EINVAL);
        }
        let res = self
            .task
            .mmap(addr, length, prot, flags, fd, offset)
            .map(|addr| addr as isize);
        res
    }

    pub fn sys_munmap(&self, start: usize, length: usize) -> SyscallResult {
        self.task
            .memory_set()
            .lock()
            .mmap_manager
            .remove(VirtAddr(start), length);
        Ok(0)
    }

    // mprotect 226
    pub fn sys_mprotect(&self, addr: usize, length: usize, prot: usize) -> SyscallResult {
        let root_ppn = Arch::current_root_ppn();
        let page_table = PageTable::from_ppn(root_ppn);

        let map_flags_raw = ((prot & 0b111) << 1) + (1 << 4);
        let map_perm = MapPermission::from_bits(map_flags_raw).unwrap();
        let mapping_flags: MappingFlags = map_perm.into();

        let start_va = VirtAddr::from(addr);
        let end_va = VirtAddr::from(addr + length);
        let vpn_range = VpnRange::new_from_va(start_va, end_va);
        if !start_va.is_aligned() {
            return_errno!(Errno::EINVAL);
        }

        info!(
            "[sys_mprotect] range: {:?}, map_perm: {:?}, mapping_flags: {:?}",
            vpn_range, map_perm, mapping_flags
        );

        for vpn in vpn_range {
            if let Some(pte) = page_table.find_pte(vpn) {
                let flags = pte.flags().union(mapping_flags);
                pte.set_flags(flags);
                debug!(
                    "[sys_mprotect] set flags in page table, vpn: {:#x}, flags: {:?}, pte_raw: {:#x}",
                    vpn.0, flags, pte.0
                );
            } else {
                let task = self.task;
                let mut memory_set = task.memory_set().lock();
                let mmap_start = memory_set.mmap_manager.mmap_start;
                let mmap_top = memory_set.mmap_manager.mmap_top;
                let mmap_perm = MmapProts::from_bits(prot).unwrap();
                let va: VirtAddr = vpn.into();
                if va >= mmap_start && va < mmap_top {
                    memory_set
                        .mmap_manager
                        .mmap_map
                        .get_mut(&vpn)
                        .ok_or(Errno::ENOMEM)?
                        .prot = mmap_perm;
                    continue;
                }
                // fixme: we don't actually set flags in mmap's mprotect
                return_errno!(Errno::EINVAL, "invalid vpn: {:?}", vpn);
            }
        }
        Arch::tlb_flush();
        Ok(0)
    }
}

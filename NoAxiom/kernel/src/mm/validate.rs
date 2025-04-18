use alloc::sync::Arc;

use arch::{Arch, ArchMemory, ArchPageTableEntry, MappingFlags, PageTableEntry, TrapType};
use ksync::mutex::SpinLock;
use memory::address::VirtAddr;

use super::{address::VirtPageNum, memory_set::MemorySet};
use crate::{
    cpu::current_task,
    include::result::Errno,
    mm::{mmap_manager::lazy_alloc_mmap, page_table::PageTable},
    syscall::SysResult,
    task::Task,
};

// TODO: add mmap check
/// # memory validate
/// Check if is the copy-on-write/lazy-alloc pages triggered the page fault.
///
/// As for cow, clone pages for the writer(aka current task),
/// but should keep original page as cow since it might still be shared.
/// Note that if the reference count is one, there's no need to clone pages.
///
/// As for lazy alloc, realloc pages for the task.
/// Associated pages: stack, heap, mmap
///
/// Return value: true if successfully handled lazy alloc or copy-on-write;
///               false if the page fault is not in any alloc area.
///
/// usages: when any kernel allocation in user_space happens, call this fn;
/// when user pagefault happens, call this func to check allocation.
pub async fn validate(
    memory_set: &Arc<SpinLock<MemorySet>>,
    vpn: VirtPageNum,
    trap_type: Option<TrapType>,
    pte: Option<PageTableEntry>,
    is_blocked: bool,
) -> SysResult<()> {
    if let Some(pte) = pte {
        let flags = pte.flags();
        if flags.contains(MappingFlags::COW) {
            info!(
                "[memory_validate] realloc COW at vpn: {:#x}, pte: {:#x}, flags: {:?}, tid: {}",
                vpn.0,
                pte.0,
                pte.flags(),
                current_task().tid()
            );
            memory_set.lock().realloc_cow(vpn, pte);
            Arch::tlb_flush();
            Ok(())
        } else if trap_type.is_some() && matches!(trap_type.unwrap(), TrapType::StorePageFault(_)) {
            error!(
                "[memory_validate] store at invalid area, flags: {:?}, tid: {}",
                flags,
                current_task().tid()
            );
            Err(Errno::EFAULT)
        } else {
            error!("unknown error in memory validate, flag: {:?}", flags);
            Err(Errno::EFAULT)
        }
    } else {
        let mut ms = memory_set.lock();
        if ms.user_stack_area.vpn_range.is_in_range(vpn) {
            let task = current_task();
            info!(
                "[memory_validate] realloc stack, tid: {}, addr: {:#x?}, epc: {:#x}",
                task.tid(),
                vpn.0,
                task.trap_context()[arch::TrapArgs::EPC],
            );
            ms.lazy_alloc_stack(vpn);
            Arch::tlb_flush();
            Ok(())
        } else if ms.user_brk_area.vpn_range.is_in_range(vpn) {
            info!(
                "[memory_validate] realloc heap, tid: {}, addr: {:x?}",
                current_task().tid(),
                vpn.0
            );
            ms.lazy_alloc_brk(vpn);
            Arch::tlb_flush();
            Ok(())
        } else {
            info!(
                "[memory_validate] realloc mmap, tid: {}, addr: {:x?}",
                current_task().tid(),
                vpn.0
            );
            lazy_alloc_mmap(memory_set, vpn, ms).await?;
            Arch::tlb_flush();
            Ok(())
        }
    }
}

impl Task {
    pub async fn memory_validate(
        self: &Arc<Self>,
        addr: usize,
        trap_type: Option<TrapType>,
        is_blocked: bool,
    ) -> SysResult<()> {
        trace!("[memory_validate] check at addr: {:#x}", addr);
        let vpn = VirtAddr::from(addr).floor();
        let pt = PageTable::from_ppn(Arch::current_root_ppn());
        validate(
            self.memory_set(),
            vpn,
            trap_type,
            pt.translate_vpn(vpn),
            is_blocked,
        )
        .await
    }
}

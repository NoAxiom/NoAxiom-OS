use alloc::sync::Arc;

use arch::{Arch, ArchMemory, ArchPageTableEntry, MappingFlags, PageTableEntry, TrapType};
use ksync::mutex::SpinLock;
use memory::address::VirtAddr;

use super::{address::VirtPageNum, memory_set::MemorySet};
use crate::{
    cpu::current_task,
    include::result::Errno,
    mm::{memory_set::lazy_alloc_mmap, page_table::PageTable},
    syscall::SysResult,
    task::Task,
};

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
    pte: Option<&mut PageTableEntry>,
) -> SysResult<()> {
    if let Some(pte) = pte {
        let flags = pte.flags();
        if flags.contains(MappingFlags::COW) {
            trace!("[validate] realloc COW, vpn={:#x}", vpn.raw());
            memory_set.lock().realloc_cow(vpn, pte)?;
            Ok(())
        } else if trap_type.is_some() && matches!(trap_type.unwrap(), TrapType::StorePageFault(_)) {
            error!(
                "[validate] store at invalid area, flags: {:?}, tid: {}",
                flags,
                current_task().unwrap().tid(),
            );
            Err(Errno::EFAULT)
        } else {
            error!(
                "[validate] unknown error, vpn: {:#x}, flag: {:?}",
                vpn.raw(),
                flags
            );
            Err(Errno::EFAULT)
        }
    } else {
        let mut ms = memory_set.lock();
        if ms.stack.vpn_range.is_in_range(vpn) {
            let task = current_task().unwrap();
            trace!(
                "[validate] stack, tid: {}, vpn: {:#x?}, epc: {:#x}",
                task.tid(),
                vpn.raw(),
                task.trap_context()[arch::TrapArgs::EPC],
            );
            ms.lazy_alloc_stack(vpn);
            Ok(())
        } else if ms.brk.area.vpn_range.is_in_range(vpn) {
            trace!(
                "[validate] brk, tid: {}, vpn: {:x?}, epc: {:#x}",
                current_task().unwrap().tid(),
                vpn.raw(),
                current_task().unwrap().trap_context()[arch::TrapArgs::EPC],
            );
            ms.lazy_alloc_brk(vpn);
            Ok(())
        } else if ms.mmap_manager.is_in_space(vpn) {
            trace!(
                "[validate] mmap, tid: {}, vpn: {:x?}, epc: {:#x}",
                current_task().unwrap().tid(),
                vpn.raw(),
                current_task().unwrap().trap_context()[arch::TrapArgs::EPC],
            );
            drop(ms);
            lazy_alloc_mmap(memory_set, vpn).await?;
            Ok(())
        } else {
            error!("[validate] not in any area, vpn: {:#x}", vpn.raw());
            Err(Errno::EFAULT)
        }
    }
}

impl Task {
    pub async fn memory_validate(
        self: &Arc<Self>,
        addr: usize,
        trap_type: Option<TrapType>,
        is_blockon: bool,
    ) -> SysResult<()> {
        trace!(
            "[memory_validate] {:x?} at user_pc={:#x}, is_kernel: {}",
            trap_type,
            self.trap_context()[arch::TrapArgs::EPC],
            is_blockon,
        );
        let ms = self.memory_set();
        let vpn = VirtAddr::from(addr).floor();
        let pt = PageTable::from_ppn(Arch::current_root_ppn());
        let pte = pt.find_pte(vpn);
        validate(ms, vpn, trap_type, pte).await
    }
}

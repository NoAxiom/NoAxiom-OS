use alloc::sync::Arc;

use arch::{ArchPageTableEntry, MappingFlags, PageTableEntry, TrapType};
use ksync::mutex::SpinLock;

use super::{address::VirtPageNum, memory_set::MemorySet};
use crate::{
    cpu::current_cpu, include::result::Errno, mm::mmap_manager::lazy_alloc_mmap, syscall::SysResult,
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
) -> SysResult<()> {
    if let Some(pte) = pte {
        let flags = pte.flags();
        if flags.contains(MappingFlags::COW) {
            trace!(
                "[memory_validate] realloc COW at vpn: {:#x}, pte: {:#x}, flags: {:?}, tid: {}",
                vpn.0,
                pte.0,
                pte.flags(),
                current_cpu().task.as_ref().unwrap().tid()
            );
            memory_set.lock().realloc_cow(vpn, pte);
            Ok(())
        } else if trap_type.is_some() && matches!(trap_type.unwrap(), TrapType::StorePageFault(_)) {
            error!(
                "[memory_validate] store at invalid area, flags: {:?}, tid: {}",
                flags,
                current_cpu().task.as_ref().unwrap().tid()
            );
            Err(Errno::EFAULT)
        } else {
            error!("unknown error in memory validate, flag: {:?}", flags);
            Err(Errno::EFAULT)
        }
    } else {
        let mut ms = memory_set.lock();
        if ms.user_stack_area.vpn_range.is_in_range(vpn) {
            let task = current_cpu().task.as_ref().unwrap();
            info!(
                "[memory_validate] realloc stack, tid: {}, addr: {:#x?}, epc: {:#x}",
                task.tid(),
                vpn,
                task.trap_context()[arch::TrapArgs::EPC],
            );
            ms.lazy_alloc_stack(vpn);
            Ok(())
        } else if ms.user_brk_area.vpn_range.is_in_range(vpn) {
            info!(
                "[memory_validate] realloc heap, tid: {}, addr: {vpn:#x?}",
                current_cpu().task.as_ref().unwrap().tid()
            );
            ms.lazy_alloc_brk(vpn);
            Ok(())
        } else {
            info!(
                "[memory_validate] realloc mmap, tid: {}, addr: {vpn:#x?}",
                current_cpu().task.as_ref().unwrap().tid()
            );
            lazy_alloc_mmap(memory_set, vpn, ms).await?;
            Ok(())
        }
    }
}

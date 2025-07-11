use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use arch::ArchPageTableEntry;
use include::errno::Errno;

use super::{
    address::{VirtAddr, VirtPageNum, VpnRange},
    frame::FrameTracker,
    page_table::PageTable,
};
use crate::{
    config::mm::{MMAP_BASE_ADDR, PAGE_SIZE},
    fs::vfs::basic::file::File,
    include::mm::{MmapFlags, MmapProts},
    pte_flags,
    syscall::SysResult,
    with_interrupt_on,
};

/// single mmap page struct
#[derive(Clone)]
pub struct MmapPage {
    /// mmap protection
    pub prot: MmapProts,

    /// mmap flags
    pub flags: MmapFlags,

    /// validity, indicating whether the page is acutally mapped
    pub valid: bool,

    /// mmapped file
    pub file: Option<Arc<dyn File>>,

    /// offset in file
    pub offset: usize,
}

impl MmapPage {
    /// mmap alloc
    pub async fn lazy_map_page(&mut self, kernel_vpn: VirtPageNum) -> SysResult<()> {
        if let Some(file) = self.file.clone() {
            let buf_slice: &mut [u8] = unsafe {
                core::slice::from_raw_parts_mut(kernel_vpn.as_va_usize() as *mut u8, PAGE_SIZE)
            };
            warn!("mmap read file, offset: {:#x}", self.offset);
            let res = with_interrupt_on!(file.read_at(self.offset, buf_slice).await);
            if let Err(res) = res {
                error!("ERROR at mmap read file, msg: {:?}", res);
            }
        }
        self.valid = true;
        Ok(())
    }
}

pub struct MmapManager {
    /// base of mmap space
    pub mmap_start: VirtAddr,

    /// top of mmap space
    pub mmap_top: VirtAddr,

    /// mmap space, containing all mmap pages whenever they are allocated or not
    pub mmap_map: BTreeMap<VirtPageNum, MmapPage>,

    /// frame trackers for already allocated mmap pages
    pub frame_trackers: BTreeMap<VirtPageNum, FrameTracker>,
}

impl Clone for MmapManager {
    fn clone(&self) -> Self {
        Self {
            mmap_start: self.mmap_start,
            mmap_top: self.mmap_top,
            mmap_map: self.mmap_map.clone(),
            frame_trackers: BTreeMap::new(),
        }
    }
}

impl MmapManager {
    pub fn new(mmap_start: VirtAddr, mmap_top: VirtAddr) -> Self {
        Self {
            mmap_start,
            mmap_top,
            mmap_map: BTreeMap::new(),
            frame_trackers: BTreeMap::new(),
        }
    }

    pub fn new_bare() -> Self {
        Self::new(
            VirtAddr::from(MMAP_BASE_ADDR),
            VirtAddr::from(MMAP_BASE_ADDR),
        )
    }

    /// push a mmap range in mmap space (not actually mapped)
    pub fn insert(
        &mut self,
        start_va: VirtAddr,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        st_offset: usize,
        file: Option<Arc<dyn File>>,
    ) -> SysResult<usize> {
        let end_va = VirtAddr::from(start_va.raw() + length);
        let mut offset = st_offset;
        for vpn in VpnRange::new_from_va(start_va, end_va)? {
            // created a mmap page with lazy-mapping
            let mmap_page = MmapPage {
                prot,
                flags,
                valid: false,
                file: file.clone(),
                offset,
            };
            self.mmap_map.insert(vpn, mmap_page);
            offset += PAGE_SIZE;
        }
        if self.mmap_top <= start_va {
            self.mmap_top = (start_va.raw() + length).into();
        }
        Ok(start_va.raw())
    }

    /// remove a mmap range in mmap space
    pub fn remove(&mut self, start_va: VirtAddr, length: usize) -> SysResult<()> {
        let end_va = VirtAddr::from(start_va.raw() + length);
        for vpn in VpnRange::new_from_va(start_va, end_va)? {
            self.mmap_map.remove(&vpn);
            self.frame_trackers.remove(&vpn);
        }
        Ok(())
    }

    /// is a va in mmap space
    pub fn is_in_space(&self, vpn: VirtPageNum) -> bool {
        self.mmap_map.contains_key(&vpn)
    }

    /// mprotect
    pub fn mprotect(
        &mut self,
        vpn: VirtPageNum,
        add_prot: MmapProts,
        page_table: &PageTable,
    ) -> SysResult<()> {
        let page = self.mmap_map.get_mut(&vpn).ok_or(Errno::ENOMEM)?;
        let old_prot = page.prot;
        let new_prot = old_prot | add_prot;
        if self.frame_trackers.contains_key(&vpn) {
            if let Some(pte) = page_table.find_pte(vpn) {
                let flags = pte_flags!(U) | new_prot.into();
                pte.set_flags(flags);
            } else {
                warn!(
                    "[mprotect] not in table, vpn: {:#x}, old_prot: {:?}, add_prot: {:?}",
                    vpn.raw(),
                    old_prot,
                    add_prot
                );
            }
        }
        page.prot = new_prot;
        Ok(())
    }
}

/*
pub async fn lazy_alloc_mmap<'a>(
    memory_set: &Arc<SpinLock<MemorySet>>,
    vpn: VirtPageNum,
    mut guard: SpinLockGuard<'a, MemorySet>,
) -> SysResult<()> {
    let frame = frame_alloc().unwrap();
    let ppn = frame.ppn();
    let kernel_vpn = frame.into_kernel_vpn();
    guard.mmap_manager.frame_trackers.insert(vpn, frame);
    let mmap_page = guard.mmap_manager.mmap_map.remove(&vpn);
    match mmap_page {
        Some(mut mmap_page) => {
            drop(guard);
            let pte_flags: MappingFlags = MappingFlags::from(mmap_page.prot) | MappingFlags::U;
            mmap_page.lazy_map_page(kernel_vpn).await?;
            let mut ms = memory_set.lock();
            ms.page_table().map(vpn, ppn, pte_flags);
            if let Some(tracer) = ms.mmap_manager.alloc_tracer.get_mut(&vpn) {
                for waker in tracer.iter() {
                    waker.wake_by_ref();
                }
                ms.mmap_manager.alloc_tracer.remove(&vpn);
            }
            assert!(ms.mmap_manager.mmap_map.get(&vpn).is_none());
            assert!(ms.mmap_manager.alloc_tracer.get(&vpn).is_none());
            ms.mmap_manager.mmap_map.insert(vpn, mmap_page);
            drop(ms);
            Ok(())
        }
        None => match guard.mmap_manager.alloc_tracer.get_mut(&vpn) {
            Some(tracer) => {
                unimplemented!();
                tracer.push(take_waker().await);
                drop(guard);
                debug!("[lazy_alloc_mmap] suspend_no_int_now");
                loop {
                    suspend_no_int_now(current_task().unwrap().pcb()).await;
                    if memory_set.lock().mmap_manager.mmap_map.get(&vpn).is_some() {
                        break;
                    }
                }
                Ok(())
            }
            None => {
                error!(
                    "[lazy_alloc_mmap] vpn not found in mmap_map, vpn: {:#x}",
                    vpn.0
                );
                Err(Errno::EFAULT)
            }
        },
    }
}

*/

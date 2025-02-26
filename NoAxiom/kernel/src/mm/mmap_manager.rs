use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use super::{
    address::{VirtAddr, VirtPageNum, VpnRange},
    frame::FrameTracker,
};
use crate::{
    config::mm::PAGE_SIZE,
    fs::vfs::basic::file::File,
    include::mm::{MmapFlags, MmapProts},
};

#[derive(Clone)]
pub struct MmapPage {
    /// Starting virtual address of mmap space
    pub vpn: VirtPageNum,
    /// Mmap space validity
    pub valid: bool,
    /// Mmap space permissions
    pub prot: MmapProts,
    /// Mapping flags
    pub flags: MmapFlags,
    /// File descriptor
    pub file: Option<Arc<dyn File>>,
    /// Mapped file offset address
    pub offset: usize,
}

impl MmapPage {
    pub fn new(
        vpn: VirtPageNum,
        prot: MmapProts,
        flags: MmapFlags,
        valid: bool,
        file: Option<Arc<dyn File>>,
        offset: usize,
    ) -> Self {
        Self {
            vpn,
            prot,
            flags,
            valid,
            file,
            offset,
        }
    }
}

pub struct MmapManager {
    pub mmap_start: VirtAddr,
    pub mmap_top: VirtAddr,
    pub mmap_map: BTreeMap<VirtPageNum, MmapPage>,
    pub frame_trackers: BTreeMap<VirtPageNum, FrameTracker>,
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
    pub fn push(
        &mut self,
        start_va: VirtAddr,
        len: usize,
        prot: MmapProts,
        flags: MmapFlags,
        offset: usize,
        file: Option<Arc<dyn File>>,
    ) -> usize {
        let end_va = VirtAddr(start_va.0 + len);
        // use lazy map
        let mut offset = offset;
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            debug!("[DEBUG] mmap map vpn:{:x?}", vpn);
            let mmap_page = MmapPage::new(vpn, prot, flags, false, file.clone(), offset);
            self.mmap_map.insert(vpn, mmap_page);
            offset += PAGE_SIZE;
        }
        // update mmap_top
        if self.mmap_top <= start_va {
            self.mmap_top = (start_va.0 + len).into();
        }
        start_va.0
    }
    pub fn remove(&mut self, start_va: VirtAddr, len: usize) {
        let end_va = VirtAddr(start_va.0 + len);
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            self.mmap_map.remove(&vpn);
            self.frame_trackers.remove(&vpn);
        }
    }
}

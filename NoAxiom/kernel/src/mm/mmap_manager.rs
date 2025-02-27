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

/// single mmap page struct
#[derive(Clone)]
pub struct MmapPage {
    /// base va of mmap space
    pub vpn: VirtPageNum,

    /// validity
    pub valid: bool,

    /// mmap protection
    pub prot: MmapProts,

    /// mmap flags
    pub flags: MmapFlags,

    /// mmapped file
    pub file: Option<Arc<dyn File>>,

    /// offset in file
    pub offset: usize,
}

impl MmapPage {
    /// register a new mmap page without immediate mapping
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

    // pub async fn lazy_map_page(&mut self, token: usize) {
    //     if self.flags.contains(MmapFlags::MAP_ANONYMOUS) {
    //         self.read_from_zero(token);
    //     } else {
    //         self.read_from_file(token).await;
    //     }
    //     self.valid = true;
    // }

    // fn read_from_zero(&mut self, token: usize) {
    //     UserBuffer::wrap(translated_bytes_buffer(
    //         token,
    //         VirtAddr::from(self.vpn).0 as *const u8,
    //         PAGE_SIZE,
    //     ))
    //     .write_zeros();
    // }
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

    /// push a mmap range in mmap space (not actually mapped)
    pub fn push(
        &mut self,
        start_va: VirtAddr,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        st_offset: usize,
        file: Option<Arc<dyn File>>,
    ) -> usize {
        let end_va = VirtAddr(start_va.0 + length);
        let mut cur_offset = st_offset;
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            // created a mmap page without mapping
            let mmap_page = MmapPage::new(vpn, prot, flags, false, file.clone(), cur_offset);
            self.mmap_map.insert(vpn, mmap_page);
            cur_offset += PAGE_SIZE;
        }
        if self.mmap_top <= start_va {
            self.mmap_top = (start_va.0 + length).into();
        }
        start_va.0
    }

    /// remove a mmap range in mmap space
    pub fn remove(&mut self, start_va: VirtAddr, length: usize) {
        let end_va = VirtAddr(start_va.0 + length);
        for vpn in VpnRange::new_from_va(start_va, end_va) {
            self.mmap_map.remove(&vpn);
            self.frame_trackers.remove(&vpn);
        }
    }
}

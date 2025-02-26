use alloc::{collections::btree_map::BTreeMap, sync::Arc};

use super::{
    address::{VirtAddr, VirtPageNum},
    frame::FrameTracker,
};
use crate::{
    fs::vfs::basic::file::File,
    include::mm::{MmapFlags, MmapProts},
};

/// Mmap Block
///
/// Used to record information about mmap space. Mmap data is not stored here.
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
}

//! map area

use alloc::collections::btree_map::BTreeMap;

use super::{
    address::{VirtAddr, VirtPageNum, VpnRange},
    frame::{frame_alloc, FrameTracker},
    page_table::PageTable,
    permission::{MapPermission, MapType},
    pte::PTEFlags,
};
use crate::{config::mm::PAGE_SIZE, mm::address::StepOne};

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAreaType {
    UserStack,
    UserHeap,
    ElfBinary,
    File,
    KernelSpace,
}

// TODO: file & file offset
/// map area, saves program data mapping information
pub struct MapArea {
    /// The range of the virtual page number
    pub vpn_range: VpnRange,

    /// program data frame tracker holder,
    /// mapping from vpn to ppn
    /// TODO: should it be Arc<FrameTracker>?
    pub frame_map: BTreeMap<VirtPageNum, FrameTracker>,

    /// address mapping type
    pub map_type: MapType,

    /// the permission of the map area
    pub map_permission: MapPermission,

    /// area type
    pub area_type: MapAreaType,
}

impl MapArea {
    /// create a new map area
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_permission: MapPermission,
        map_area_type: MapAreaType,
    ) -> Self {
        Self {
            vpn_range: VpnRange::new_from_va(start_va, end_va),
            frame_map: BTreeMap::new(),
            map_permission,
            map_type,
            area_type: map_area_type,
        }
    }

    /// create new from another map area
    pub fn from_another(other: &MapArea) -> Self {
        Self {
            vpn_range: other.vpn_range.clone(),
            frame_map: BTreeMap::new(),
            map_permission: other.map_permission.clone(),
            map_type: other.map_type.clone(),
            area_type: other.area_type.clone(),
        }
    }

    /// get vpn range
    pub fn vpn_range(&self) -> VpnRange {
        self.vpn_range.clone()
    }

    /// for each vpn in the range,
    /// map the vpn to ppn
    /// pte will be saved into page_table
    /// and data frame will be saved by self
    pub fn map_each(&mut self, page_table: &mut PageTable) {
        trace!("map_each: vpn_range = {:?}", self.vpn_range);
        match self.map_type {
            MapType::Identical => {
                panic!("kernel don't support identical memory mapping");
            }
            // framed: user space
            MapType::Framed => {
                for vpn in self.vpn_range.into_iter() {
                    let frame = frame_alloc().unwrap();
                    let ppn = frame.ppn;
                    if self.frame_map.contains_key(&vpn) {
                        panic!("vm area overlap");
                    }
                    self.frame_map.insert(vpn, frame);
                    let flags = PTEFlags::from_bits(self.map_permission.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                    assert!(page_table.find_pte(vpn).is_some());
                }
            }
            // direct: kernel space
            MapType::Direct => {
                for vpn in self.vpn_range.into_iter() {
                    // trace!("map_each: vpn = {:#x}", vpn.0);
                    let ppn = vpn.kernel_translate_into_ppn();
                    // let ppn = PhysPageNum(vpn.0 - KERNEL_ADDR_OFFSET);
                    let flags = PTEFlags::from_bits(self.map_permission.bits()).unwrap();
                    page_table.map(vpn, ppn, flags);
                }
            }
        }
    }

    // TODO: offset
    /// load data from byte slice
    pub fn load_data(&mut self, page_table: &PageTable, data: &[u8]) {
        // should only load user data
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate_vpn(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

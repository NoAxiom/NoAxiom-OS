//! map area

use alloc::collections::btree_map::BTreeMap;

use arch::{Arch, ArchMemory, ArchPageTableEntry};

use super::{
    address::{VirtAddr, VirtPageNum, VpnRange},
    frame::{frame_alloc, FrameTracker},
    page_table::PageTable,
    permission::{MapPermission, MapType},
};
use crate::{
    config::mm::PAGE_SIZE,
    mm::address::{PhysPageNum, StepOne},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAreaType {
    None,
    UserStack,
    UserHeap,
    ElfBinary,
    KernelSpace,
    Shared,
}

/// map area, saves program data mapping information
pub struct MapArea {
    /// The range of the virtual page number
    pub vpn_range: VpnRange,

    /// program data frame tracker holder,
    /// mapping from vpn to ppn
    /// use Arc because we share it in copy-on-write fork
    pub frame_map: BTreeMap<VirtPageNum, FrameTracker>,

    /// address mapping type
    pub map_type: MapType,

    /// the permission of the map area
    pub map_permission: MapPermission,

    /// area type
    pub area_type: MapAreaType,
}

impl MapArea {
    pub fn new_bare() -> Self {
        Self {
            vpn_range: VpnRange::new(VirtPageNum(0), VirtPageNum(0)),
            frame_map: BTreeMap::new(),
            map_permission: MapPermission::empty(),
            map_type: MapType::Identical,
            area_type: MapAreaType::None,
        }
    }

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
            // file: None,
            // file_offset: 0,
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
            // file: other.file.clone(),
            // file_offset: other.file_offset,
        }
    }

    /// get vpn range
    pub fn vpn_range(&self) -> VpnRange {
        self.vpn_range.clone()
    }

    /// map one page at `vpn`
    pub fn map_one(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        // trace!(
        //     "map_one: vpn = {:#x}, ppn = {:#x}, flags = {:?}",
        //     vpn.0, ppn.0, flags
        // );
        match self.map_type {
            MapType::Identical => {
                panic!("kernel don't support identical memory mapping");
            }
            // framed: user space
            MapType::Framed => {
                let frame = frame_alloc();
                let ppn = frame.ppn();
                if self.frame_map.contains_key(&vpn) {
                    panic!("vm area overlap");
                }
                self.frame_map.insert(vpn, frame);
                let flags = self.map_permission.into();
                page_table.map(vpn, ppn, flags);
                assert!(page_table.find_pte(vpn).is_some());
            }
            // direct: kernel space
            MapType::Direct => {
                let ppn = vpn.kernel_translate_into_ppn();
                let flags = self.map_permission.into();
                page_table.map(vpn, ppn, flags);
            }
        }
    }

    /// for each vpn in the range, map the vpn to ppn
    /// pte will be saved into page_table
    /// and data frame will be saved by self
    pub fn map_each(&mut self, page_table: &mut PageTable) {
        trace!(
            "map_each: va_range = {:?}, ppn_range = [{:#x},{:#x}), type: {:?}",
            self.vpn_range,
            self.vpn_range.start().kernel_translate_into_ppn().0,
            self.vpn_range.end().kernel_translate_into_ppn().0,
            self.map_type
        );
        for vpn in self.vpn_range.into_iter() {
            self.map_one(vpn, page_table);
        }
    }

    /// unmap one page at `vpn`
    pub fn unmap_one(&mut self, vpn: VirtPageNum, page_table: &mut PageTable) {
        trace!("unmap_one: vpn = {:?}", vpn);
        match self.map_type {
            MapType::Identical => {
                panic!("kernel don't support identical memory mapping");
            }
            MapType::Framed => {
                self.frame_map.remove(&vpn);
                page_table.unmap(vpn);
            }
            MapType::Direct => {
                page_table.unmap(vpn);
            }
        }
    }

    /// modify end vpn of current map area
    pub fn change_end_vpn(&mut self, new_end_vpn: VirtPageNum, page_table: &mut PageTable) {
        let old_end_vpn = self.vpn_range.end();
        self.vpn_range = VpnRange::new(self.vpn_range.start(), new_end_vpn);
        trace!(
            "[change_end_vpn]: old: {:#x}, new: {:#x}",
            old_end_vpn.0,
            new_end_vpn.0
        );
        if new_end_vpn < old_end_vpn {
            debug!(
                "[change_end_vpn] remove pages in [{:#x}, {:#x})",
                new_end_vpn.0, old_end_vpn.0
            );
            for vpn in VpnRange::new(new_end_vpn, old_end_vpn).into_iter() {
                self.frame_map.remove(&vpn);
                self.unmap_one(vpn, page_table);
            }
            Arch::tlb_flush();
        }
    }

    /// load data from byte slice
    pub fn load_data(&mut self, page_table: &PageTable, data: &[u8], offset: usize) {
        // should only load user data
        assert_eq!(self.map_type, MapType::Framed);
        let mut cur_st: usize = 0;
        let mut current_vpn = self.vpn_range.start();
        let len = data.len();
        if offset != 0 {
            let src = &data[0..len.min(PAGE_SIZE - offset)];
            cur_st += PAGE_SIZE - offset;
            let ppn = PhysPageNum::from(page_table.translate_vpn(current_vpn).unwrap().ppn());
            let dst = &mut ppn.get_bytes_array()[offset..src.len() + offset];
            dst.copy_from_slice(src);
            current_vpn.step();
        }
        while cur_st < len {
            let src = &data[cur_st..len.min(cur_st + PAGE_SIZE)];
            cur_st += PAGE_SIZE;
            let ppn = PhysPageNum::from(page_table.translate_vpn(current_vpn).unwrap().ppn());
            let dst = &mut ppn.get_bytes_array()[0..src.len()];
            dst.copy_from_slice(src);
            current_vpn.step();
        }
        trace!(
            "[load_data]: cur_st = {:#x}, area: {:?}",
            cur_st,
            self.vpn_range
        );
    }
}

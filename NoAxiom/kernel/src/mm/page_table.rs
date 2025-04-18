//! page table under sv39

use alloc::vec::Vec;

use arch::{
    consts::INDEX_LEVELS, Arch, ArchMemory, ArchPageTableEntry, MappingFlags, PageTableEntry,
};

use super::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    frame::{frame_alloc, FrameTracker},
};
use crate::pte_flags;

#[derive(Debug)]
pub struct PageTable {
    /// root ppn, serves as an identifier of this page table
    root_ppn: PhysPageNum,

    /// page table frame tracker holder,
    /// doesn't track data pages
    frames: Vec<FrameTracker>,

    /// is kernel?
    is_kernel: bool,
}

impl PageTable {
    /// create a new page table without any allocation
    /// SAFETY: this function is only act as a placeholder,
    /// don't really use this to construct a page table
    pub fn new_bare() -> Self {
        PageTable {
            root_ppn: PhysPageNum(0),
            frames: Vec::new(),
            is_kernel: false,
        }
    }

    /// create a new page table,
    /// with allocating a frame for root node
    /// used in raw memory_set initialization
    pub fn new_allocated() -> Self {
        let frame = frame_alloc();
        info!("[page_table] root_ppn = {:#x}", frame.ppn().0);
        PageTable {
            root_ppn: frame.ppn(),
            frames: vec![frame],
            is_kernel: false,
        }
    }

    pub fn mark_as_kernel(&mut self) {
        self.is_kernel = true;
    }

    /// use ppn to generate a new pagetable,
    /// note that the frame won't be saved,
    /// so do assure that it's already wrapped in tcb
    pub fn from_ppn(ppn: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(ppn),
            frames: Vec::new(),
            is_kernel: false,
        }
    }

    /// clone from another page table, only direct page will be copied
    pub fn clone_from_other(other: &PageTable) -> Self {
        let new_frame = frame_alloc();
        new_frame
            .ppn()
            .get_bytes_array()
            .copy_from_slice(other.root_ppn.get_bytes_array());
        PageTable {
            root_ppn: new_frame.ppn(),
            frames: vec![new_frame],
            is_kernel: false,
        }
    }

    /// insert new pte into the page table trie
    fn create_pte(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
        let index = vpn.get_index();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in index.iter().enumerate() {
            let arr = ppn.get_pte_array();
            let pte = &mut arr[*idx];
            if i == INDEX_LEVELS - 1 {
                result = Some(pte);
                break;
            }
            trace!("pte addr: {:#x}", pte as *mut PageTableEntry as usize);
            if !pte.is_valid_dir() {
                let frame = frame_alloc();
                *pte = PageTableEntry::new(frame.ppn().0, pte_flags!(V, PT));
                self.frames.push(frame);
            }
            ppn = PhysPageNum::from(pte.ppn());
        }
        result.unwrap()
    }

    /// try to find pte, returns None at failure
    #[inline(always)]
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        translate_vpn_into_pte(self.root_ppn, vpn)
    }

    /// map vpn -> ppn
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: MappingFlags) {
        let pte = self.create_pte(vpn);
        assert!(
            (!pte.flags().contains(MappingFlags::V)),
            "{:#x?} is mapped before mapping, flags: {:?}, ppn: {:#x}",
            vpn,
            pte.flags(),
            pte.ppn()
        );
        trace!(
            "mapping: vpn: {:#x?}, ppn: {:#x?}, flags: {:?}, pte_addr: {:#x}",
            vpn,
            ppn,
            flags,
            pte as *mut PageTableEntry as usize
        );
        *pte = PageTableEntry::new(ppn.0, flags | pte_flags!(V, D, A));

        let find_res = self.find_pte(vpn).unwrap();
        assert!(
            find_res.flags().contains(MappingFlags::V),
            "error vpn: {:#x}, flags: {:?}",
            vpn.0,
            find_res.flags()
        );
    }

    /// unmap a vpn
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        if !pte.flags().contains(MappingFlags::V) {
            error!("{:?} is invalid before unmapping", vpn);
        }
        pte.reset();
    }

    /// translate vpn into pte
    /// returns None if nothing is mapped
    pub fn translate_vpn(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// translate va into pa
    /// returns None if nothing is mapped
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        let vpn = va.clone().floor();
        let res = self.find_pte(vpn);
        res.map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    #[allow(unused)]
    pub fn translate_va_debug(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            debug!(
                "translate_va_debug: va: {:#x}, pa: {:#x}, offset: {:#x}",
                va.0, aligned_pa_usize, offset
            );
            (aligned_pa_usize + offset).into()
        })
    }

    /// get root ppn
    #[inline(always)]
    pub const fn root_ppn(&self) -> PhysPageNum {
        self.root_ppn
    }

    /// set flags for a vpn
    pub fn set_flags(&mut self, vpn: VirtPageNum, flags: MappingFlags) {
        self.find_pte(vpn).unwrap().set_flags(flags);
    }

    /// switch into this page table,
    /// PLEASE make sure context around is mapped into both page tables
    #[inline(always)]
    pub fn memory_activate(&self) {
        Arch::activate(self.root_ppn().0, self.is_kernel);
    }

    /// remap a cow page
    pub fn remap_cow(
        &mut self,
        vpn: VirtPageNum,
        ppn: PhysPageNum,
        old_ppn: PhysPageNum,
        new_flags: MappingFlags,
    ) {
        let pte = self.create_pte(vpn);
        *pte = PageTableEntry::new(ppn.0, new_flags);
        ppn.get_bytes_array()
            .copy_from_slice(old_ppn.get_bytes_array());
    }
}

// pub fn memory_activate_by_ppn(root_ppn: usize) {
//     Arch::activate(root_ppn);
//     Arch::tlb_flush();
// }

/// translate the vpn into PTE entry (sv39)
/// it won't use memory_set to translate the vpn
/// note that this is read only
pub fn translate_vpn_into_pte<'a>(
    root_ppn: PhysPageNum,
    vpn: VirtPageNum,
) -> Option<&'a mut PageTableEntry> {
    let index = vpn.get_index();
    let mut ppn = root_ppn;
    let mut result: Option<&mut PageTableEntry> = None;
    for (i, idx) in index.iter().enumerate() {
        let pte = &mut ppn.get_pte_array()[*idx];
        if !pte.is_valid_dir() {
            return None;
        }
        if i == INDEX_LEVELS - 1 {
            result = Some(pte);
            break;
        }
        ppn = pte.ppn().into();
    }
    result
}

#[inline(always)]
pub fn flags_switch_to_cow(flags: &MappingFlags) -> MappingFlags {
    *flags & !MappingFlags::W | MappingFlags::COW
}
#[inline(always)]
pub fn flags_switch_to_rw(flags: &MappingFlags) -> MappingFlags {
    *flags & !MappingFlags::COW | MappingFlags::W
}

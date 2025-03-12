//! page table under sv39

use alloc::vec::Vec;
use core::arch::asm;

use arch::{Arch, ArchMemory};

use super::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    frame::{frame_alloc, FrameTracker},
    pte::{PTEFlags, PageTableEntry},
};
use crate::{config::mm::PPN_MASK, pte_flags};

#[derive(Debug)]
pub struct PageTable {
    /// root ppn, serves as an identifier of this page table
    root_ppn: PhysPageNum,

    /// page table frame tracker holder,
    /// doesn't track data pages
    frames: Vec<FrameTracker>,
}

impl PageTable {
    /// create a new page table without any allocation
    /// SAFETY: this function is only act as a placeholder,
    /// don't really use this to construct a page table
    pub fn new_bare() -> Self {
        PageTable {
            root_ppn: PhysPageNum(0),
            frames: Vec::new(),
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
        }
    }

    /// use satp[43:0] to generate a new pagetable,
    /// note that the frame won't be saved,
    /// so do assure that it's already wrapped in tcb
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & PPN_MASK),
            frames: Vec::new(),
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
        }
    }

    /// insert new pte into the page table trie
    fn create_pte(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
        // debug!("insert: vpn = {:#x}", vpn.0);
        let index = vpn.get_index();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in index.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.flags().is_valid() {
                let frame = frame_alloc();
                *pte = PageTableEntry::new(frame.ppn(), pte_flags!(V));
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result.unwrap()
    }

    /// try to find pte, returns None at failure
    #[inline(always)]
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        translate_vpn_into_pte(self.root_ppn, vpn)
    }

    /// map vpn -> ppn
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.create_pte(vpn);
        assert!(
            !pte.flags().is_valid(),
            "{:?} is mapped before mapping",
            vpn
        );
        *pte = PageTableEntry::new(ppn, flags | pte_flags!(V, D, A));

        // let find_res = self.find_pte(vpn).unwrap();
        // assert!(
        //     find_res.flags().is_valid(),
        //     "error vpn: {:#x}, flags: {:?}",
        //     vpn.0,
        //     find_res.flags()
        // );
    }

    /// unmap a vpn
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        if !pte.flags().is_valid() {
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
            info!(
                "translate_va_debug: va: {:#x}, pa: {:#x}, offset: {:#x}, pte: {:#x?}",
                va.0, aligned_pa_usize, offset, pte
            );
            (aligned_pa_usize + offset).into()
        })
    }

    /// get the token of this page table (WARNING: sv39 only)
    /// which will be written into satp
    #[inline(always)]
    pub const fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }

    /// set flags for a vpn
    pub fn set_flags(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        self.create_pte(vpn).set_flags(flags);
    }

    /// switch into this page table,
    /// PLEASE make sure context around is mapped into both page tables
    #[inline(always)]
    pub unsafe fn memory_activate(&self) {
        memory_activate_by_token(self.token());
    }

    /// remap a cow page
    pub fn remap_cow(
        &mut self,
        vpn: VirtPageNum,
        ppn: PhysPageNum,
        old_ppn: PhysPageNum,
        new_flags: PTEFlags,
    ) {
        let pte = self.create_pte(vpn);
        *pte = PageTableEntry::new(ppn, new_flags);
        ppn.get_bytes_array()
            .copy_from_slice(old_ppn.get_bytes_array());
    }
}

pub fn memory_activate_by_token(token: usize) {
    Arch::update_pagetable(token);
    Arch::tlb_flush();
}

pub fn current_token() -> usize {
    let satp: usize;
    unsafe { asm!("csrr {}, satp", out(reg) satp) }
    satp
}

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
        if !pte.flags().is_valid() {
            return None;
        }
        if i == 2 {
            result = Some(pte);
            break;
        }
        ppn = pte.ppn();
    }
    result
}

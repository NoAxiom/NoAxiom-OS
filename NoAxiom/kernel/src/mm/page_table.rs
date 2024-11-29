//! page table under sv39

use alloc::vec::Vec;
use core::arch::asm;

use riscv::register::satp;

use super::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    frame::{frame_alloc, FrameTracker},
    pte::{PTEFlags, PageTableEntry},
};
use crate::pte_flags;

#[derive(Debug)]
pub struct PageTable {
    /// root ppn, serves as an identifier of this page table
    root_ppn: PhysPageNum,

    /// saves all frame trackers to preserve its life cycle
    frames: Vec<FrameTracker>,
}

impl PageTable {
    /// create a new page table,
    /// with allocating a frame for root node
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// use satp[43:0] to generate a new pagetable,
    /// note that the frame won't be saved,
    /// so do assure that it's already wrapped in tcb
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// insert new pte into the page table trie
    fn insert_pte(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
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
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, pte_flags!(V));
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    /// try to find pte, returns None at failure
    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let index = vpn.get_index();
        let mut ppn = self.root_ppn;
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

    /// map vpn -> ppn
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.insert_pte(vpn).unwrap();
        assert!(
            !pte.flags().is_valid(),
            "{:?} is mapped before mapping",
            vpn
        );
        *pte = PageTableEntry::new(ppn, flags | pte_flags!(V, D, A));
    }

    /// unmap a vpn
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(
            pte.flags().is_valid(),
            "{:?} is invalid before unmapping",
            vpn
        );
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
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    /// get the token of this page table (sv39 only)
    pub fn token(&self) -> usize {
        // sv39
        8usize << 60 | self.root_ppn.0
    }

    /// set copy-on-write for a vpn
    pub fn set_cow(&mut self, vpn: VirtPageNum) {
        self.insert_pte(vpn).unwrap().set_cow();
    }

    /// reset copy-on-write for a vpn
    pub fn reset_cow(&mut self, vpn: VirtPageNum) {
        self.insert_pte(vpn).unwrap().reset_cow();
    }

    /// set flags for a vpn
    pub fn set_flags(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        self.insert_pte(vpn).unwrap().set_flags(flags);
    }

    /// remap a vpn with new ppn
    pub fn remap_cow(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, former_ppn: PhysPageNum) {
        let pte = self.insert_pte(vpn).unwrap();
        *pte = PageTableEntry::new(ppn, pte.flags() | pte_flags!(W));
        ppn.get_bytes_array()
            .copy_from_slice(former_ppn.get_bytes_array());
    }

    /// switch into this page table,
    /// PLEASE make sure the context is mapped in both page tables
    pub unsafe fn activate(&self) {
        let satp: usize = self.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
}

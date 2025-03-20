use config::mm::PAGE_WIDTH;
use loongArch64::register::pgdl;

use super::LA64;
use crate::{utils::macros::bit, ArchMemory, ArchPageTable, ArchPageTableEntry, MappingFlags};

const PA_WIDTH: usize = 56;
const VA_WIDTH: usize = 39;
const INDEX_LEVELS: usize = 3;

impl From<MappingFlags> for PTEFlags {
    fn from(value: MappingFlags) -> Self {
        let mut flags = PTEFlags::V;
        if value.contains(MappingFlags::W) {
            flags |= PTEFlags::W | PTEFlags::D;
        }
        // if !value.contains(MappingFlags::X) {
        //     flags |= PTEFlags::NX;
        // }
        if value.contains(MappingFlags::U) {
            flags |= PTEFlags::PLV_USER;
        }
        flags
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(val: PTEFlags) -> Self {
        let mut flags = MappingFlags::empty();
        if val.contains(PTEFlags::W) {
            flags |= MappingFlags::W;
        }
        if val.contains(PTEFlags::D) {
            flags |= MappingFlags::D;
        }
        // if !self.contains(PTEFlags::NX) {
        //     flags |= MappingFlags::X;
        // }
        if val.contains(PTEFlags::PLV_USER) {
            flags |= MappingFlags::U;
        }
        flags
    }
}

bitflags::bitflags! {
    /// Possible flags for a page table entry.
    pub struct PTEFlags: u64 {
        /// Page Valid
        const V = bit!(0);

        /// Dirty, The page has been writed.
        const D = bit!(1);

        const PLV_USER = 0b11 << 2;

        const MAT_NOCACHE = 0b01 << 4;

        /// Designates a global mapping OR Whether the page is huge page.
        const GH = bit!(6);

        /// Page is existing.
        const P = bit!(7);

        /// Page is writeable.
        const W = bit!(8);

        /// Is a Global Page if using huge page(GH bit).
        const G = bit!(10);

        /// Page is not readable.
        const NR = bit!(11);

        /// Page is not executable.
        /// FIXME: Is it just for a huge page?
        /// Linux related url: https://github.com/torvalds/linux/blob/master/arch/loongarch/include/asm/pgtable-bits.h
        const NX = bit!(12);

        /// Whether the privilege Level is restricted. When RPLV is 0, the PTE
        /// can be accessed by any program with privilege Level highter than PLV.
        const RPLV = bit!(63);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(pub usize);

impl Into<usize> for PageTableEntry {
    fn into(self) -> usize {
        self.0
    }
}
impl From<usize> for PageTableEntry {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

const FLAG_WIDTH: usize = 12;
const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH;
impl ArchPageTableEntry for PageTableEntry {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self {
        let flags = PTEFlags::from(flags);
        Self((ppn << FLAG_WIDTH) | flags.bits() as usize)
    }
    /// get the physical page number
    fn ppn(&self) -> usize {
        (self.0 >> FLAG_WIDTH) & ((1usize << PPN_WIDTH) - 1)
    }
    /// get the pte permission flags
    fn flags(&self) -> MappingFlags {
        PTEFlags::from_bits((self.0 & ((1 << FLAG_WIDTH) - 1)) as u64)
            .unwrap()
            .into()
    }
    /// set flags
    fn set_flags(&mut self, flags: MappingFlags) {
        let flags = PTEFlags::from(flags);
        self.0 = (self.0 & !((1 << FLAG_WIDTH) - 1)) | (flags.bits() as usize);
    }
    /// clear all data
    fn reset(&mut self) {
        self.0 = 0;
    }
}

/// flush the TLB entry by VirtualAddress
/// currently unused
#[inline]
#[allow(unused)]
pub fn flush_vaddr(va: usize) {
    unsafe {
        core::arch::asm!("dbar 0; invtlb 0x05, $r0, {reg}", reg = in(reg) va);
    }
}

#[inline]
pub fn tlb_flush_all_with_dbar() {
    unsafe {
        core::arch::asm!("dbar 0; invtlb 0x00, $r0, $r0");
    }
}

fn activate_ppn(ppn: usize) {
    pgdl::set_base(ppn << PAGE_WIDTH);
}

pub struct PageTable(pub usize);

impl ArchPageTable for PageTable {
    type PageTableEntry = PageTableEntry;
    const PA_WIDTH: usize = PA_WIDTH;
    const VA_WIDTH: usize = VA_WIDTH;
    const INDEX_LEVELS: usize = INDEX_LEVELS;

    fn new(root_ppn: usize) -> Self {
        Self(root_ppn)
    }
    fn root_ppn(&self) -> usize {
        self.0
    }
    fn activate(&self) {
        activate_ppn(self.0);
    }
}

impl ArchMemory for LA64 {
    type PageTable = PageTable;
    fn tlb_flush() {
        // fixme: is this tlbflush or dbar?
        tlb_flush_all_with_dbar();
    }
    fn activate(ppn: usize) {
        activate_ppn(ppn);
    }
    fn current_root_ppn() -> usize {
        pgdl::read().base() >> PAGE_WIDTH
    }
}

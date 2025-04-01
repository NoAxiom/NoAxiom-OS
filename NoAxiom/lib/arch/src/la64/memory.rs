use config::mm::PAGE_WIDTH;
use loongArch64::register::{pgdh, pgdl};

use super::{
    tlb::{tlb_refill, tlb_flush_all, tlb_init_inner},
    LA64,
};
use crate::{utils::macros::bit, ArchMemory, ArchPageTable, ArchPageTableEntry, MappingFlags};

const PA_WIDTH: usize = 56;
const VA_WIDTH: usize = 39;
const INDEX_LEVELS: usize = 3;
const PHYS_MEMORY_START: usize = 0x9000_0000;
const MEMORY_SIZE: usize = 0x1000_0000;
const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
pub const KERNEL_ADDR_OFFSET: usize = 0x9000_0000_0000_0000;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        const G = bit!(12);

        /// Page is not readable.
        const NR = bit!(61);

        /// Page is not executable.
        /// Linux related url: https://github.com/torvalds/linux/blob/master/arch/loongarch/include/asm/pgtable-bits.h
        const NX = bit!(62);

        /// Whether the privilege Level is restricted. When RPLV is 0, the PTE
        /// can be accessed by any program with privilege Level highter than PLV.
        const RPLV = bit!(63);
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        let mut res = PTEFlags::empty();
        // V D U P W G?? NX
        if flags.contains(MappingFlags::V) {
            res |= PTEFlags::V;
            res |= PTEFlags::P;
        }
        if flags.contains(MappingFlags::X) {
            res |= PTEFlags::NX;
        }
        if flags.contains(MappingFlags::W) {
            res |= PTEFlags::W;
        }
        if flags.contains(MappingFlags::D) {
            res |= PTEFlags::D;
        }
        if flags.contains(MappingFlags::U) {
            res |= PTEFlags::PLV_USER;
        }
        res
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(val: PTEFlags) -> Self {
        let mut res = MappingFlags::empty();
        // V D U P W G?? NX
        // log::debug!("PTEFlags: {:?}", val);
        if val.contains(PTEFlags::V) && val.contains(PTEFlags::P) {
            res |= MappingFlags::V;
        }
        if val.contains(PTEFlags::W) {
            res |= MappingFlags::W;
        }
        if val.contains(PTEFlags::D) {
            res |= MappingFlags::D;
        }
        if !val.contains(PTEFlags::NX) {
            res |= MappingFlags::X;
        }
        if val.contains(PTEFlags::PLV_USER) {
            res |= MappingFlags::U;
        }
        res
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
const PPN_MASK: usize = (1usize << PPN_WIDTH) - 1;
impl ArchPageTableEntry for PageTableEntry {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self {
        let flags = PTEFlags::from(flags);
        Self((ppn << FLAG_WIDTH) | flags.bits() as usize)
    }
    /// get the physical page number
    fn ppn(&self) -> usize {
        (self.0 >> FLAG_WIDTH) & PPN_MASK
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

fn low_activate_ppn(ppn: usize) {
    pgdl::set_base(ppn << PAGE_WIDTH);
}

fn high_activate_ppn(ppn: usize) {
    pgdh::set_base(ppn << PAGE_WIDTH);
}

pub struct PageTable(pub usize);

impl ArchPageTable for PageTable {
    type PageTableEntry = PageTableEntry;
    const VA_WIDTH: usize = VA_WIDTH;
    const INDEX_LEVELS: usize = INDEX_LEVELS;

    fn new(root_ppn: usize) -> Self {
        Self(root_ppn)
    }
    fn root_ppn(&self) -> usize {
        self.0
    }
    fn activate(&self) {
        low_activate_ppn(self.0);
    }
}

pub(crate) fn tlb_init() {
    tlb_init_inner(tlb_refill as _);
    tlb_flush_all();
}

impl ArchMemory for LA64 {
    const PHYS_MEMORY_START: usize = PHYS_MEMORY_START;
    const PHYS_MEMORY_END: usize = PHYS_MEMORY_END;
    const KERNEL_ADDR_OFFSET: usize = KERNEL_ADDR_OFFSET;
    type PageTable = PageTable;
    fn tlb_init() {
        tlb_init();
    }
    fn tlb_flush() {
        tlb_flush_all();
    }
    fn activate(ppn: usize, is_kernel: bool) {
        match is_kernel {
            true => high_activate_ppn(ppn),
            false => low_activate_ppn(ppn),
        }
        tlb_flush_all();
    }
    fn current_root_ppn() -> usize {
        // todo: when in kernel space, it's incorrect
        pgdl::read().base() >> PAGE_WIDTH
    }
}

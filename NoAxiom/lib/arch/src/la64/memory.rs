use config::mm::PAGE_WIDTH;
use loongArch64::register::{dmw2, pgd, pgdh, pgdl, MemoryAccessType};

use super::{
    tlb::{tlb_flush_all, tlb_init_inner},
    LA64,
};
use crate::{ArchMemory, ArchPageTable, ArchPageTableEntry, MappingFlags};

const PA_WIDTH: usize = 48;
const VA_WIDTH: usize = 48;
const INDEX_LEVELS: usize = 4;
const PHYS_MEMORY_START: usize = 0x9000_0000;
const MEMORY_SIZE: usize = 0x2000_0000;
const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
pub(crate) const KERNEL_ADDR_OFFSET: usize = 0x9000_0000_0000_0000;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    /// Possible flags for a page table entry.
    pub struct PTEFlags: usize {
        /// Valid Bit
        const V = 1 << 0;
        /// Dirty Bit, true if it is modified.
        const D = 1 << 1;
        /// Privilege Level field
        const PLV0 = 0;
        const PLV1 = 1 << 2;
        const PLV2 = 2 << 2;
        const PLV3 = 3 << 2;
        /// Memory Access Type: Strongly-ordered UnCached (SUC)
        const MAT_SUC = 0 << 4;
        /// Memory Access Type: Coherent Cached (CC)
        const MAT_CC = 1 << 4;
        /// Memory Access Type: Weakly-ordered UnCached (WUC)
        const MAT_WUC = 2 << 4;
        /// Global Bit (Basic PTE)
        const G = 1 << 6;
        /// Physical Bit, whether the physical page exists
        const P = 1 << 7;
        /// Writable Bit
        const W = 1 << 8;
        /// Copy On Write Bit (NoAxiom only)
        const COW = 1 << 9;
        /// Not Readable Bit
        const NR = 1 << (usize::BITS - 3); // 61
        /// Executable Bit
        const NX = 1 << (usize::BITS - 2); // 62
        /// Restricted Privilege LeVel enable (RPLV) for the page table.
        /// When RPLV=0, the page table entry can be accessed by any program whose privilege level is not lower than PLV;
        /// when RPLV=1, the page table entry can only be accessed by programs whose privilege level is equal to PLV.
        const RPLV = 1 << (usize::BITS - 1); // 63
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        if flags.contains(MappingFlags::PT) {
            return PTEFlags::empty();
        }
        // V D PLV P MAT W NX NR COW
        // V R X W D U COW
        let mut res = PTEFlags::P | PTEFlags::MAT_CC;
        if flags.contains(MappingFlags::V) && !flags.contains(MappingFlags::NV) {
            res |= PTEFlags::V;
        }
        if !flags.contains(MappingFlags::R) {
            res |= PTEFlags::NR;
        }
        if !flags.contains(MappingFlags::X) {
            res |= PTEFlags::NX;
        }
        if flags.contains(MappingFlags::W) {
            res |= PTEFlags::W;
        }
        if flags.contains(MappingFlags::D) {
            res |= PTEFlags::D;
        }
        if flags.contains(MappingFlags::U) {
            res |= PTEFlags::PLV3;
        }
        if flags.contains(MappingFlags::G) {
            res |= PTEFlags::G;
        }
        if flags.contains(MappingFlags::COW) {
            res |= PTEFlags::COW;
        }
        res
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(val: PTEFlags) -> Self {
        let mut res = MappingFlags::empty();
        // V  NR NX W  PLV COW D  P MAT G
        // V  R  X  W  U   COW D        G
        if val.contains(PTEFlags::V) {
            res |= MappingFlags::V;
        }
        if !val.contains(PTEFlags::NR) {
            res |= MappingFlags::R;
        }
        if !val.contains(PTEFlags::NX) {
            res |= MappingFlags::X;
        }
        if val.contains(PTEFlags::W) {
            res |= MappingFlags::W;
        }
        if val.contains(PTEFlags::PLV3) {
            res |= MappingFlags::U;
        }
        if val.contains(PTEFlags::D) {
            res |= MappingFlags::D;
        }
        if val.contains(PTEFlags::G) {
            res |= MappingFlags::G;
        }
        if val.contains(PTEFlags::COW) {
            res |= MappingFlags::COW;
        }
        res
    }
}

#[repr(C)]
#[derive(Clone)]
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
const FLAG_MASK: usize = !(PPN_MASK << FLAG_WIDTH);
impl ArchPageTableEntry for PageTableEntry {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self {
        let flags = PTEFlags::from(flags);
        Self(((ppn << FLAG_WIDTH) & !FLAG_MASK) | flags.bits() as usize)
    }
    /// get the physical page number
    fn ppn(&self) -> usize {
        (self.0 >> FLAG_WIDTH) & PPN_MASK
    }
    /// get the pte permission flags
    fn flags(&self) -> MappingFlags {
        PTEFlags::from_bits(self.0 & FLAG_MASK).unwrap().into()
    }
    /// set flags
    fn set_flags(&mut self, flags: MappingFlags) {
        let ppn = self.ppn();
        *self = Self::new(ppn, flags);
    }
    /// clear all data
    fn reset(&mut self) {
        self.0 = 0;
    }
    /// is allocated
    fn is_allocated(&self) -> bool {
        self.0 != 0
    }
}

impl PageTableEntry {
    pub fn raw_flag(&self) -> PTEFlags {
        PTEFlags::from_bits(self.0 & FLAG_MASK).unwrap()
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
    tlb_init_inner();
    tlb_flush_all();
}

#[allow(unused)]
pub(crate) fn user_trampoline_init() {
    dmw2::set_mat(MemoryAccessType::StronglyOrderedUnCached);
    dmw2::set_plv0(true);
    dmw2::set_plv3(true);
    dmw2::set_vseg(0xA);
    assert!(dmw2::read().vseg() == 0xA);
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
        pgd::read().base() >> PAGE_WIDTH
    }
}

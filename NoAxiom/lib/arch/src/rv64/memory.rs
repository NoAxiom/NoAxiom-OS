use core::arch::asm;

use bitflags::bitflags;
use config::mm::PAGE_WIDTH;
use riscv::{asm::sfence_vma_all, register::satp};

use super::RV64;
use crate::{utils::macros::bit, ArchMemory, ArchPageTable, ArchPageTableEntry, MappingFlags};

/// physical address width
pub const PA_WIDTH: usize = 56;
/// virtual address width
pub const VA_WIDTH: usize = 39;
/// index level number of sv39
pub const INDEX_LEVELS: usize = 3;
/// physical page number width
pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH; // 44
/// ppn mask
pub const PPN_MASK: usize = (1 << PPN_WIDTH) - 1;

pub const PHYS_MEMORY_START: usize = 0x8020_0000;
pub const PHYS_MEMORY_SIZE: usize = 0x4000_0000;
pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + PHYS_MEMORY_SIZE;
pub const KERNEL_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;

pub struct PageTable {
    root_ppn: usize,
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

const PTE_WIDTH: usize = 10;
impl ArchPageTableEntry for PageTableEntry {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self {
        let flags = PTEFlags::from(flags);
        Self((ppn << PTE_WIDTH) | flags.bits() as usize)
    }
    /// get the physical page number
    fn ppn(&self) -> usize {
        (self.0 >> PTE_WIDTH) & ((1usize << PPN_WIDTH) - 1)
    }
    /// get the pte permission flags
    fn flags(&self) -> MappingFlags {
        self.raw_flag().into()
    }
    /// set flags
    fn set_flags(&mut self, flags: MappingFlags) {
        let flags = PTEFlags::from(flags);
        self.0 = (self.0 & !((1 << PTE_WIDTH) - 1)) | (flags.bits() as usize);
    }
    /// clear all data
    fn reset(&mut self) {
        self.0 = 0;
    }
    /// is valid
    fn is_allocated(&self) -> bool {
        self.0 != 0
    }
}

impl PageTableEntry {
    /// raw flags
    pub fn raw_flag(&self) -> PTEFlags {
        PTEFlags::from_bits((self.0 & ((1 << PTE_WIDTH) - 1)) as u64).unwrap()
    }
}

impl ArchPageTable for PageTable {
    type PageTableEntry = PageTableEntry;
    const VA_WIDTH: usize = VA_WIDTH;
    const INDEX_LEVELS: usize = INDEX_LEVELS;
    fn new(root_ppn: usize) -> Self {
        Self { root_ppn }
    }
    fn root_ppn(&self) -> usize {
        self.root_ppn
    }
    fn activate(&self) {
        satp::write(8usize << 60 | self.root_ppn);
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PTEFlags: u64 {
        const V = bit!(0);
        const R = bit!(1);
        const W = bit!(2);
        const X = bit!(3);
        const U = bit!(4);
        const G = bit!(5);
        const A = bit!(6);
        const D = bit!(7);
        const COW = bit!(8);

        const VRWX  = Self::V.bits() | Self::R.bits() | Self::W.bits() | Self::X.bits();
        const ADUVRX = Self::A.bits() | Self::D.bits() | Self::U.bits() | Self::V.bits() | Self::R.bits() | Self::X.bits();
        const ADVRWX = Self::A.bits() | Self::D.bits() | Self::VRWX.bits();
        const ADGVRWX = Self::G.bits() | Self::ADVRWX.bits();
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        if flags.is_empty() {
            Self::empty()
        } else if flags.contains(MappingFlags::PT) {
            Self::V
        } else {
            let mut res = Self::empty();
            macro_rules! set {
                ($($flag:ident),*) => {
                    $(
                        if flags.contains(MappingFlags::$flag) {
                            res |= PTEFlags::$flag;
                        }
                    )*
                };
            }
            if flags.contains(MappingFlags::V) && !flags.contains(MappingFlags::NV) {
                res |= PTEFlags::V;
            }
            set!(R, W, X, U, G, A, D, COW);
            res
        }
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(value: PTEFlags) -> Self {
        let mut mapping_flags = MappingFlags::empty();
        macro_rules! set {
            ($($flag:ident),*) => {
                $(
                    if value.contains(PTEFlags::$flag) {
                        mapping_flags |= MappingFlags::$flag;
                    }
                )*
            };
        }
        set!(V, R, W, X, U, G, A, D, COW);
        mapping_flags
    }
}

impl ArchMemory for RV64 {
    const PHYS_MEMORY_START: usize = PHYS_MEMORY_START;
    const PHYS_MEMORY_END: usize = PHYS_MEMORY_END;
    const KERNEL_ADDR_OFFSET: usize = KERNEL_ADDR_OFFSET;
    type PageTable = PageTable;
    fn tlb_init() {}
    // flush all TLB
    #[inline(always)]
    fn tlb_flush() {
        sfence_vma_all();
    }
    #[inline(always)]
    fn current_root_ppn() -> usize {
        let satp: usize;
        unsafe { asm!("csrr {}, satp", out(reg) satp) }
        satp & PPN_MASK
    }
    #[inline(always)]
    fn activate(ppn: usize, _is_kernel: bool) {
        satp::write(8usize << 60 | ppn);
        sfence_vma_all();
    }
}

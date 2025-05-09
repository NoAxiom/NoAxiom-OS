use bitflags::bitflags;

use crate::utils::macros::bit;

// VURWXADG Device Cache Cow
bitflags! {
    /// Mapping flags for page table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MappingFlags: u64 {
        const V = bit!(0);
        const R = bit!(1);
        const W = bit!(2);
        const X = bit!(3);
        const U = bit!(4);
        const G = bit!(5);
        const A = bit!(6);
        const D = bit!(7);
        const COW = bit!(8);
        /// OS-specific: virt bit to specify the invalid page
        const NV = bit!(62);
        /// OS-specific: virt bit for page table
        const PT = bit!(63);
    }
}

pub trait ArchPageTableEntry: Into<usize> + From<usize> + Clone {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self;
    /// get the physical page number
    fn ppn(&self) -> usize;
    /// get the pte permission flags
    fn flags(&self) -> MappingFlags;
    /// set flags
    fn set_flags(&mut self, flags: MappingFlags);
    /// clear all data
    fn reset(&mut self);
    /// is valid dir
    fn is_allocated(&self) -> bool;
}

pub trait ArchPageTable {
    type PageTableEntry: ArchPageTableEntry;

    /// virtual address width
    const VA_WIDTH: usize;
    /// index level number
    const INDEX_LEVELS: usize;

    fn root_ppn(&self) -> usize;
    fn new(root_ppn: usize) -> Self;
    fn activate(&self);
}

/// memory management arch trait
pub trait ArchMemory {
    const PHYS_MEMORY_START: usize;
    const PHYS_MEMORY_END: usize;
    const KERNEL_ADDR_OFFSET: usize;
    type PageTable: ArchPageTable;
    fn tlb_init();
    fn tlb_flush();
    fn current_root_ppn() -> usize;
    fn activate(ppn: usize, is_kernel: bool);
}

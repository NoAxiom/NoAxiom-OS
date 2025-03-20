use bitflags::bitflags;
use config::mm::PAGE_WIDTH;

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

        const Device = bit!(9);
        const Cache = bit!(10);

        /// Read | Write | Executeable Flags
        const RWX = Self::R.bits() | Self::W.bits() | Self::X.bits();
        /// User | Read | Write Flags
        const URW = Self::U.bits() | Self::R.bits() | Self::W.bits();
        /// User | Read | Executeable Flags
        const URX = Self::U.bits() | Self::R.bits() | Self::X.bits();
        /// User | Read | Write | Executeable Flags
        const URWX = Self::URW.bits() | Self::X.bits();
    }
}

pub trait ArchPageTableEntry: Into<usize> + From<usize> + Clone + Copy {
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
}

macro_rules! use_self {
    ($name:ident) => {
        <Self as ArchPageTable>::$name
    };
}
/// raw vpn & ppn width
const PAGE_NUM_WIDTH: usize = PAGE_WIDTH - 3;
/// page table entry per page
const PTE_PER_PAGE: usize = 1 << PAGE_NUM_WIDTH;
pub trait ArchPageTable {
    type PageTableEntry: ArchPageTableEntry;

    /// physical address width
    const PA_WIDTH: usize;
    /// virtual address width
    const VA_WIDTH: usize;
    /// index level number
    const INDEX_LEVELS: usize;

    /// physical page number width
    const PPN_WIDTH: usize = use_self!(PA_WIDTH) - PAGE_WIDTH;
    /// ppn mask
    const PPN_MASK: usize = (1 << use_self!(PPN_WIDTH)) - 1;
    /// virtual page number width
    const VPN_WIDTH: usize = use_self!(VA_WIDTH) - PAGE_WIDTH;
    /// single pagenum width
    const PAGE_NUM_WIDTH: usize = PAGE_NUM_WIDTH;
    /// page table entry per page
    const PTE_PER_PAGE: usize = PTE_PER_PAGE;

    fn root_ppn(&self) -> usize;
    fn new(root_ppn: usize) -> Self;
    fn activate(&self);
}

/// memory management arch trait
pub trait ArchMemory {
    type PageTable: ArchPageTable;
    fn tlb_flush();
    fn current_root_ppn() -> usize;
    fn activate(ppn: usize);
    // fn update_pagetable(_bits: usize);
    // fn get_token_by_ppn(_ppn: usize) -> usize;
    // fn current_token() -> usize;
}

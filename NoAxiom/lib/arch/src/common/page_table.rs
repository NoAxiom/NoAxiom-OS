use config::mm::PAGE_WIDTH;

use super::ArchPageTableEntry;

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

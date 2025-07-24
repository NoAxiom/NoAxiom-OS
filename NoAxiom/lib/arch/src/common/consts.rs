use config::mm::PAGE_WIDTH;

use crate::{Arch, ArchMemory, ArchPageTable, VirtPageTable};

macro_rules! pt_const {
    ($const:ident) => {
        <VirtPageTable as ArchPageTable>::$const
    };
}
macro_rules! mem_const {
    ($const:ident) => {
        <Arch as ArchMemory>::$const
    };
}

/// virtual address width
pub const VA_WIDTH: usize = pt_const!(VA_WIDTH);
/// index level number
pub const INDEX_LEVELS: usize = pt_const!(INDEX_LEVELS);
/// kernel address offset from phys to virt
pub const KERNEL_ADDR_OFFSET: usize = mem_const!(KERNEL_ADDR_OFFSET);
/// kernel IO address offset from phys to virt
pub const IO_ADDR_OFFSET: usize = mem_const!(IO_ADDR_OFFSET);
/// kernel phys memory end address
pub const KERNEL_PHYS_MEMORY_END: usize = mem_const!(PHYS_MEMORY_END);

/// kernle pagenum offset from phys to virt
pub const KERNEL_PAGENUM_MASK: usize = (KERNEL_ADDR_OFFSET as isize >> PAGE_WIDTH) as usize;
/// kernel IO pagenum offset from phys to virt
pub const IO_PAGENUM_MASK: usize = (IO_ADDR_OFFSET as isize >> PAGE_WIDTH) as usize;
/// kernel virt memory end address
pub const KERNEL_VIRT_MEMORY_END: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_MEMORY_END;

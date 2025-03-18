use core::arch::asm;

use loongArch64::register::eentry;

use super::LA64;
use crate::{ArchMemory, ArchPageTable};

pub struct PageTable(pub usize);

impl ArchPageTable for PageTable {
    type PageTableEntry = usize;
    
}

impl ArchMemory for LA64 {
    type PageTable = PageTable;
    fn tlb_flush() {
        unsafe { asm!("tlbflush") };
    }
    fn activate(ppn: usize) {
        eentry::set_eentry(ppn);
    }
}

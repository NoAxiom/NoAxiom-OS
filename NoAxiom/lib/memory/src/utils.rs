use arch::consts::{KERNEL_ADDR_OFFSET, KERNEL_PAGENUM_MASK};

/// translate a raw usize type kernel virt address into phys address
#[inline(always)]
pub fn kernel_va_to_pa(virt: usize) -> usize {
    assert!(
        (virt & KERNEL_ADDR_OFFSET) == KERNEL_ADDR_OFFSET,
        "invalid kernel virt address"
    );
    virt & !KERNEL_ADDR_OFFSET
}

/// translate a raw usize type kernel phys address into virt address
#[inline(always)]
pub fn kernel_pa_to_va(phys: usize) -> usize {
    phys | KERNEL_ADDR_OFFSET
}

#[inline(always)]
pub fn kernel_vpn_to_ppn(vpn: usize) -> usize {
    vpn & !KERNEL_PAGENUM_MASK
}

#[inline(always)]
pub fn kernel_ppn_to_vpn(ppn: usize) -> usize {
    ppn | KERNEL_PAGENUM_MASK
}

pub fn print_mem_info() {
    crate::heap::print_heap_info_simple();
    crate::frame::print_frame_info_simple();
}

use arch::consts::{IO_PAGENUM_MASK, KERNEL_ADDR_OFFSET, KERNEL_PAGENUM_MASK};

/// translate a raw usize type kernel virt address into phys address
#[inline(always)]
pub fn kernel_va_to_pa(virt: usize) -> usize {
    assert!(
        (virt & KERNEL_ADDR_OFFSET) == KERNEL_ADDR_OFFSET,
        "invalid kernel virt address"
    );
    virt & !KERNEL_ADDR_OFFSET
}

#[inline(always)]
pub fn kernel_vpn_to_ppn(vpn: usize) -> usize {
    vpn & !KERNEL_PAGENUM_MASK
}

pub fn kernel_iovpn_to_ppn(vpn: usize) -> usize {
    vpn & !IO_PAGENUM_MASK
}

#[inline(always)]
pub fn kernel_ppn_to_vpn(ppn: usize) -> usize {
    ppn | KERNEL_PAGENUM_MASK
}

pub fn print_mem_info() {
    crate::heap::print_heap_info_simple();
    crate::frame::print_frame_info_simple();
}

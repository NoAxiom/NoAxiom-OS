use arch::consts::KERNEL_ADDR_OFFSET;

/// Device Tree Base Address, at loongarch64 is a constant
pub fn translate_dtb(dtb: usize) -> usize {
    // fixme this logic is error
    if dtb & KERNEL_ADDR_OFFSET == 0 {
        0x100000
    } else {
        dtb
    }
}

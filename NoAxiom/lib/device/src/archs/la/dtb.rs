/// Device Tree Base Address, at loongarch64 is a constant
pub fn translate_dtb(_dtb: usize) -> usize {
    0x100000
}

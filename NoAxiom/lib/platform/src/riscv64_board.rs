pub const PLATFORM: &str = "riscv64-board";

/// Device Tree Base Address, at riscv64 is read from register
pub fn get_dtb(dtb: usize) -> usize {
    dtb
}
pub const PLIC_NAME: &str = "";
pub const CLINT_NAME: &str = "";
pub const CHOSEN_NAME: &str = "";
pub const PCI_NAME: &str = "";
pub const VIRTIO_MMIO_NAME: &str = "";

pub const PCI_RANGE: (usize, usize) = (0x4_0000_0000, 0x4_0000_0000); // 64-bit MMIO space
pub const PCI_BUS_END: usize = 0xFF;

pub const MMIO_REGIONS: &[(usize, usize)] = &[];

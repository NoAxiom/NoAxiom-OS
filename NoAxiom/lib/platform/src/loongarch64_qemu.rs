pub const PLATFORM: &str = "loongarch64-qemu";

/// Device Tree Base Address, at loongarch64 is a constant
pub fn get_dtb(_dtb: usize) -> usize {
    0x100000
}
pub const PLIC_NAME: &str = "platic";
pub const CLINT_NAME: &str = "eiointc";
pub const CHOSEN_NAME: &str = "chosen";
pub const PCI_NAME: &str = "pci";
pub const VIRTIO_MMIO_NAME: &str = "virtio_mmio";

/// No initialization required Devices, but also from dtb info
pub const GED_PADDR: usize = 0x100E_001C;
pub const UART_PADDR: usize = 0x1FE0_01E0;

pub const PCI_RANGE: (usize, usize) = (0x4000_0000, 0x0002_0000);
pub const PCI_BUS_END: usize = 0x7F;

pub const MMIO_REGIONS: &[(usize, usize)] = &[
    (0x100E_0000, 0x0000_1000), // GED
    (0x1FE0_0000, 0x0000_1000), // UART
    (0x2000_0000, 0x1000_0000), // PCI
    (0x4000_0000, 0x0002_0000), // PCI RANGES
];

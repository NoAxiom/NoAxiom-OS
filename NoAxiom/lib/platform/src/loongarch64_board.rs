pub const PLATFORM: &str = "loongarch64-board";

/// Device Tree Base Address, at loongarch64 is a constant
pub fn get_dtb(_dtb: usize) -> usize {
    0 // zero is stand for unknown
}
pub const PLIC_NAME: &str = "";
pub const CLINT_NAME: &str = "";
pub const CHOSEN_NAME: &str = "";
pub const PCI_NAME: &str = "";
pub const VIRTIO_MMIO_NAME: &str = "";

/// No initialization required Devices, but also from dtb info
pub const GED_PADDR: usize = 0;
pub const UART_PADDR: usize = 0x1FE2_0000;

pub const PCI_RANGE: (usize, usize) = (0, 0);
pub const PCI_BUS_END: usize = 0;

pub const MMIO_REGIONS: &[(usize, usize)] = &[
    (0x1800_0000, 0x0200_0000),    // PCI memory ranges
    (0x1fe2_0000, 0xf),            // uart0
    (0x6000_0000, 0x2000_0000),    // PCI memory ranges
    (0xfe_0000_0000, 0x2000_0000), // PCI config space
];

pub const PCI_RANGE: (usize, usize) = (0x4000_0000, 0x4000_0000);
pub const PCI_BUS_END: usize = 0x7F;

const MMIO_REGIONS: &[(usize, usize)] = &[
    (0x0010_1000, 0x1000),      // RTC
    (0x0c00_0000, 0x21_0000),   // PLIC
    (0x1000_0000, 0x1000),      // UART
    (0x1000_1000, 0x8000),      // VirtIO
    (0x3000_0000, 0x1000_0000), // PCI config space
    (0x4000_0000, 0x4000_0000), // PCI memory ranges (ranges 1: 32-bit MMIO space)
    (0x16020000, 0x10000),      // SDIO
];

pub fn get_mmio_regions() -> &'static [(usize, usize)] {
    MMIO_REGIONS
}

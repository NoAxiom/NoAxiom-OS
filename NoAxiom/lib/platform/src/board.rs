// can be read from dtb
pub const MMIO_REGIONS: [(usize, usize); 4] = [
    (0x1800_0000, 0x0200_0000),    // PCI memory ranges
    (0x1fe2_0000, 0xf),            // uart0
    (0x6000_0000, 0x2000_0000),    // PCI memory ranges
    (0xfe_0000_0000, 0x2000_0000), // PCI config space
];

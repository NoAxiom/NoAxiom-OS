pub const PLATFORM: &str = "riscv64-qemu";

/// Device Tree Base Address, at riscv64 is read from register
pub fn get_dtb(dtb: usize) -> usize {
    dtb
}
pub const PLIC_NAME: &str = "plic";
pub const CLINT_NAME: &str = "eiointc";
pub const CHOSEN_NAME: &str = "chosen";
pub const PCI_NAME: &str = "pci";
pub const VIRTIO_MMIO_NAME: &str = "virtio_mmio";

/// No initialization required Devices
pub const GED_PADDR: usize = 0; // No GED on QEMU at riscv64
pub const UART_PADDR: usize = 0; // No UART on QEMU at riscv64

pub const PCI_RANGE: (usize, usize) = (0x4000_0000, 0x0002_0000);
pub const PCI_BUS_END: usize = 0x7F;

pub const MMIO_REGIONS: &[(usize, usize)] = &[
    (0x0010_1000, 0x1000),      // RTC
    (0x0c00_0000, 0x21_0000),   // PLIC
    (0x1000_0000, 0x1000),      // UART
    (0x1000_1000, 0x8000),      // VirtIO
    (0x3000_0000, 0x1000_0000), // PCI config space
    (0x4000_0000, 0x4000_0000), // PCI memory ranges (ranges 1: 32-bit MMIO space)
];

use config::mm::PAGE_WIDTH;

use crate::{Arch, ArchMemory, ArchPageTable, VirtPageTable};

macro_rules! pt_const {
    ($const:ident) => {
        <VirtPageTable as ArchPageTable>::$const
    };
}
macro_rules! mem_const {
    ($const:ident) => {
        <Arch as ArchMemory>::$const
    };
}

/// virtual address width
pub const VA_WIDTH: usize = pt_const!(VA_WIDTH);
/// index level number
pub const INDEX_LEVELS: usize = pt_const!(INDEX_LEVELS);
/// kernel address offset from phys to virt
pub const KERNEL_ADDR_OFFSET: usize = mem_const!(KERNEL_ADDR_OFFSET);
/// kernel phys memory end address
pub const KERNEL_PHYS_MEMORY_END: usize = mem_const!(PHYS_MEMORY_END);

/// kernle pagenum offset from phys to virt
pub const KERNEL_PAGENUM_MASK: usize = (KERNEL_ADDR_OFFSET as isize >> PAGE_WIDTH) as usize;
/// kernel virt memory end address
pub const KERNEL_VIRT_MEMORY_END: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_MEMORY_END;

/// qemu virtio disk mmio
pub const VIRTIO0: usize = 0x1000_1000 + KERNEL_ADDR_OFFSET;

/// MMIO on Qemu of VirtIO.
#[cfg(target_arch = "riscv64")]
pub const MMIO: &[(usize, usize)] = &[
    (0x1000_1000, 0x1000),   // VIRTIO0
    (0xc00_0000, 0x21_0000), /* VIRT_PLIC in virt machine */
    /* (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
     * (0x2000000, 0x10000),
     * (0x1000_0000, 0x9000),   // VIRT_UART0 with GPU  in virt machine
     * (0x3000_0000, 0x1000_0000),
     * */
    // (0x1000_4000, 0x4000),
];

#[cfg(target_arch = "loongarch64")]
pub const MMIO: &[(usize, usize)] = &[
    (0x100E_0000, 0x0000_1000), // GED
    (0x1FE0_0000, 0x0000_1000), // UART
    (0x2000_0000, 0x1000_0000), // PCI
    (0x4000_0000, 0x0002_0000), /* PCI RANGES */
];

// #[cfg(feature = "riscv_qemu")]
// pub const MMIO: &[(usize, usize)] = &[
//     (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
//     (0x2000000, 0x10000),
//     (0xc00_0000, 0x21_0000), // VIRT_PLIC in virt machine
//     (0x1000_0000, 0x9000),   // VIRT_UART0 with GPU  in virt machine
//     (0x3000_0000, 0x1000_0000),
// ];

// pub const MMIO: &[(usize, usize)] = &[
// (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
// (0x2000000, 0x10000),
// (0xc00_0000, 0x21_0000), // VIRT_PLIC in virt machine
// (0x1000_0000, 0x9000),   // VIRT_UART0 with GPU  in virt machine
// (0x3000_0000, 0x1000_0000),
// ];
//
// VF2 MMIO
// #[cfg(feature = "vf2")]
// pub const MMIO: &[(usize, usize)] = &[
// (0x17040000, 0x10000),  // RTC
// (0xc000000, 0x4000000), // PLIC
// (0x1000_0000, 0x10000), // UART
// (0x16020000, 0x10000),  // sdio1
// ];
//

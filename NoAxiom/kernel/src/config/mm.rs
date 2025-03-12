//! Memory management configuration

/// inner page offset witdh
pub const PAGE_WIDTH: usize = config::mm::PAGE_WIDTH;
/// page size : 4KB, 4096 bytes
pub const PAGE_SIZE: usize = config::mm::PAGE_SIZE;

/// kernel stack width
pub const KERNEL_STACK_WIDTH: usize = config::mm::KERNEL_STACK_WIDTH;
/// kernel stack size: 64KB (*CPU_NUM = 128KB)
pub const KERNEL_STACK_SIZE: usize = config::mm::KERNEL_STACK_SIZE;
/// kernel heap size: 32MB
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

/// user app's stack size: 8KB
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;
/// user app's heap size: 120MB
pub const USER_HEAP_SIZE: usize = PAGE_SIZE * 30000;

/// kernel address offset from phys to virt
pub const KERNEL_ADDR_OFFSET: usize = config::mm::KERNEL_ADDR_OFFSET;
/// kernle pagenum offset from phys to virt
pub const KERNEL_PAGENUM_MASK: usize = config::mm::KERNEL_PAGENUM_MASK;

/// mmap start address
pub const MMAP_BASE_ADDR: usize = 0x6000_0000;
/// mmap area max size
pub const MMAP_MAX_SIZE: usize = 0x1000_0000;
/// mmap max_end address
pub const MMAP_MAX_END_ADDR: usize = MMAP_BASE_ADDR + MMAP_MAX_SIZE;

// /// kernel phys memory start address
// pub const KERNEL_PHYS_ENTRY: usize = 0x8020_0000;
// /// kernel virt memory start address
// pub const KERNEL_VIRT_ENTRY: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_ENTRY;

/// kernel phys memory end address
pub const KERNEL_PHYS_MEMORY_END: usize = 0x8800_0000;
/// kernel virt memory end address
pub const KERNEL_VIRT_MEMORY_END: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_MEMORY_END;

mod sv39 {
    use super::PAGE_WIDTH; // 12

    /// physical address width
    pub const PA_WIDTH: usize = 56;
    /// virtual address width
    pub const VA_WIDTH: usize = 39;

    /// physical page number width
    pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH; // 44
    /// ppn mask
    pub const PPN_MASK: usize = (1 << PPN_WIDTH) - 1;
    /// virtual page number width
    pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_WIDTH; // 27

    /// index level number of sv39
    pub const INDEX_LEVELS: usize = 3;
    /// raw vpn & ppn width of sv39
    pub const PAGE_NUM_WIDTH: usize = VPN_WIDTH / INDEX_LEVELS; // 9
    /// page table entry per page
    pub const PTE_PER_PAGE: usize = 1 << PAGE_NUM_WIDTH; // 512
}
pub use sv39::*;

/// Dynamic linked interpreter address range in user space
pub const DL_INTERP_OFFSET: usize = 0x20_0000_0000;

/// qemu virtio disk mmio
pub const VIRTIO0: usize = 0x1000_1000 + KERNEL_ADDR_OFFSET;

/// MMIO on Qemu of VirtIO.
#[cfg(feature = "riscv_qemu")]
pub const MMIO: &[(usize, usize)] = &[
    (0x1000_1000, 0x1000), // VIRTIO0
    (0xc00_0000, 0x21_0000), /* VIRT_PLIC in virt machine */
    /* (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    * (0x2000000, 0x10000),
    * (0x1000_0000, 0x9000),   // VIRT_UART0 with GPU  in virt machine
    * (0x3000_0000, 0x1000_0000),
    */
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

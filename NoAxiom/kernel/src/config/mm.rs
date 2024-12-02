//! Memory management configuration

/// inner page offset witdh
pub const PAGE_WIDTH: usize = 12;
/// page size : 4KB, 4096 bytes
pub const PAGE_SIZE: usize = 1 << PAGE_WIDTH;

/// boot stack, only used when kernel initializes
pub const BOOT_STACK_WIDTH: usize = 16;
/// boot stack size: 16KB
pub const BOOT_STACK_SIZE: usize = 1 << BOOT_STACK_WIDTH;

/// user app's stack size: 8KB
pub const USER_STACK_SIZE: usize = 4096 * 2;
/// user app's heap size: 120MB
pub const USER_HEAP_SIZE: usize = 4096 * 30000;

/// kernel stack size: 8KB
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
/// kernel heap size: 32MB
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

/// kernel address offset from phys to virt
pub const KERNEL_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
/// kernle pagenum offset from phys to virt
pub const KERNEL_PAGENUM_OFFSET: usize = KERNEL_ADDR_OFFSET >> PAGE_WIDTH;

/// kernel phys memory start address
pub const KERNEL_PHYS_MEMORY_START: usize = 0x8020_0000;
/// kernel virt memory start address
pub const KERNEL_VIRT_MEMORY_START: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_MEMORY_START;

/// kernel phys memory end address
pub const KERNEL_PHYS_MEMORY_END: usize = 0x8800_0000;
/// kernel virt memory end address
pub const KERNEL_VIRT_MEMORY_END: usize = KERNEL_ADDR_OFFSET | KERNEL_PHYS_MEMORY_END;

#[cfg(feature = "sv39")]
mod sv39 {
    use super::PAGE_WIDTH; // 12

    /// physical address width
    pub const PA_WIDTH: usize = 56;
    /// virtual address width
    pub const VA_WIDTH: usize = 39;

    /// physical page number width
    pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH; // 44
    /// virtual page number width
    pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_WIDTH; // 27

    /// index level number of sv39
    pub const INDEX_LEVELS: usize = 3;
    /// raw vpn & ppn width of sv39
    pub const PAGE_NUM_WIDTH: usize = VPN_WIDTH / INDEX_LEVELS; // 9
    /// page table entry per page
    pub const PTE_PER_PAGE: usize = 1 << PAGE_NUM_WIDTH; // 512
}
#[cfg(feature = "sv39")]
pub use sv39::*;

#[cfg(feature = "sv48")]
mod sv48 {
    use super::PAGE_WIDTH; // 12

    /// physical address width
    pub const PA_WIDTH: usize = 56;
    /// virtual address width
    pub const VA_WIDTH: usize = 48;

    /// physical page number width
    pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH; // 44
    /// virtual page number width
    pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_WIDTH; // 36

    /// index level number of sv48
    pub const INDEX_LEVELS: usize = 4;
    /// raw vpn & ppn width of sv48
    pub const PAGE_NUM_WIDTH: usize = VPN_WIDTH / INDEX_LEVELS; // 9
    /// page table entry per page
    pub const PTE_PER_PAGE: usize = 1 << PAGE_NUM_WIDTH; // 512
}
#[cfg(feature = "sv48")]
pub use sv48::*;

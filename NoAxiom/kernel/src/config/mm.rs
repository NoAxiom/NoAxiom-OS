//! memory management configs

/// user app's stack size
pub const USER_STACK_SIZE: usize = 4096 * 2;

/// kernel stack size
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;

/// kernel heap size
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

/// physical memory end address
pub const MEMORY_END: usize = 0x8800_0000;

/// inner page offset witdh
pub const PAGE_WIDTH: usize = 12;

/// page size : 4KB
pub const PAGE_SIZE: usize = 1 << PAGE_WIDTH;

/// the max number of syscall
pub const MAX_SYSCALL_NUM: usize = 500;

/// the virtual addr of trapoline
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

/// the virtual addr of trap context
pub const TRAP_CONTEXT_BASE: usize = TRAMPOLINE - PAGE_SIZE;

mod sv39 {
    use super::PAGE_WIDTH;

    /// physical address width
    pub const PA_WIDTH: usize = 56;

    /// virtual address width
    pub const VA_WIDTH: usize = 39;

    /// physical page number width
    pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_WIDTH; // 44

    /// virtual page number width
    pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_WIDTH; // 27
}
pub use sv39::*;

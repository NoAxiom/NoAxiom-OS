//! Memory management configuration

/// inner page offset witdh
pub const PAGE_WIDTH: usize = 12;
/// page size : 4KB, 4096 bytes
pub const PAGE_SIZE: usize = 1 << PAGE_WIDTH;

/// kernel stack width
/// fixme: reset to 16 later?
pub const KERNEL_STACK_WIDTH: usize = 20;
/// kernel stack size: 64KB (*CPU_NUM = 128KB)
pub const KERNEL_STACK_SIZE: usize = 1 << KERNEL_STACK_WIDTH;
/// kernel heap size: 32MB
pub const KERNEL_HEAP_SIZE: usize = 0x300_0000;

/// raw vpn & ppn width: 9
pub const PAGE_NUM_WIDTH: usize = PAGE_WIDTH - 3;
/// page table entry per page: 512
pub const PTE_PER_PAGE: usize = 1 << PAGE_NUM_WIDTH;

/// user app's stack size: 8MB
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2048;
/// user app's heap size: 120MB
pub const USER_HEAP_SIZE: usize = PAGE_SIZE * 30000;

/// mmap start address
pub const MMAP_BASE_ADDR: usize = 0x6_0000_0000;
/// mmap area max size
pub const MMAP_MAX_SIZE: usize = 0x1000_0000;
/// mmap max_end address
pub const MMAP_MAX_END_ADDR: usize = MMAP_BASE_ADDR + MMAP_MAX_SIZE;

/// user memory end address
pub const USER_MEMORY_END: usize = 0x20_0000_0000;

/// share memory offset
pub const SHM_OFFSET: usize = 0x5_0000_0000;

/// Dynamic linked interpreter address range in user space
pub const DL_INTERP_OFFSET: usize = USER_MEMORY_END + 0x10_0000_0000;

/// signal trampoline address
pub const SIG_TRAMPOLINE: usize = USER_MEMORY_END - PAGE_SIZE;

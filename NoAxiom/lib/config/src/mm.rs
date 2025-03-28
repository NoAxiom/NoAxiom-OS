/// inner page offset witdh
pub const PAGE_WIDTH: usize = 12;
/// page size : 4KB, 4096 bytes
pub const PAGE_SIZE: usize = 1 << PAGE_WIDTH;

/// kernel stack width
pub const KERNEL_STACK_WIDTH: usize = 16;
/// kernel stack size: 64KB (*CPU_NUM = 128KB)
pub const KERNEL_STACK_SIZE: usize = 1 << KERNEL_STACK_WIDTH;

/// kernel address offset from phys to virt
#[cfg(target_arch = "riscv64")]
pub const KERNEL_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;
#[cfg(target_arch = "loongarch64")]
pub const KERNEL_ADDR_OFFSET: usize = 0x9000_0000_0000_0000;
/// kernle pagenum offset from phys to virt
pub const KERNEL_PAGENUM_MASK: usize = (KERNEL_ADDR_OFFSET as isize >> PAGE_WIDTH) as usize;

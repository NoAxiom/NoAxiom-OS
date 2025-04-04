/// inner page offset witdh
pub const PAGE_WIDTH: usize = 12;
/// page size : 4KB, 4096 bytes
pub const PAGE_SIZE: usize = 1 << PAGE_WIDTH;

/// kernel stack width
/// fixme: reset to 16 later?
pub const KERNEL_STACK_WIDTH: usize = 20;
/// kernel stack size: 64KB (*CPU_NUM = 128KB)
pub const KERNEL_STACK_SIZE: usize = 1 << KERNEL_STACK_WIDTH;

/// qemu virtio mmio, at physical address
pub const VIRTIO0: usize = 0x1000_1000;
pub const VIRTIO7: usize = 0x10007000;

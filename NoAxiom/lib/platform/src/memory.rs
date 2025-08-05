#[cfg(target_arch = "riscv64")]
mod config {
    #[cfg(feature = "qemu")]
    mod config {
        const PHYS_MEMORY_START: usize = 0x8000_0000;
        const MEMORY_SIZE: usize = 0x4000_0000;
        pub const KERNEL_HEAP_SIZE: usize = 0x1000_0000;
        pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
        pub const VALID_PHYS_CPU_MASK: usize = 0b1111;
    }
    #[cfg(not(feature = "qemu"))]
    mod config {
        const PHYS_MEMORY_START: usize = 0x4000_0000;
        const MEMORY_SIZE: usize = 0x1_0000_0000;
        pub const KERNEL_HEAP_SIZE: usize = 0x1000_0000;
        pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
        pub const VALID_PHYS_CPU_MASK: usize = 0b11110;
    }
    pub use config::*;
}

#[cfg(target_arch = "loongarch64")]
mod config {
    #[cfg(feature = "qemu")]
    mod config {
        const PHYS_MEMORY_START: usize = 0x9000_0000;
        const MEMORY_SIZE: usize = 0x2000_0000;
        pub const KERNEL_HEAP_SIZE: usize = 0x800_0000;
        pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
        pub const VALID_PHYS_CPU_MASK: usize = 0b1111;
    }
    #[cfg(not(feature = "qemu"))]
    mod config {
        const PHYS_MEMORY_START: usize = 0x9000_0000;
        const MEMORY_SIZE: usize = 0x2000_0000;
        pub const KERNEL_HEAP_SIZE: usize = 0x1000_0000;
        pub const PHYS_MEMORY_END: usize = PHYS_MEMORY_START + MEMORY_SIZE;
        pub const VALID_PHYS_CPU_MASK: usize = 0b1111;
    }
    pub use config::*;
}

pub use config::*;

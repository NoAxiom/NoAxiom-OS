//! architecture and hardware configs

/// CPU count for multi-core
#[cfg(feature = "multicore")]
pub const CPU_NUM: usize = 2;

/// CPU count, always be 1 since multicore is off
#[cfg(not(feature = "multicore"))]
pub const CPU_NUM: usize = 1;

// // loongarch64
pub const PCI_RANGE: (usize, usize) = (0x4000_0000, 0x0002_0000);
pub const PCI_BUS_END: usize = 0xFF;

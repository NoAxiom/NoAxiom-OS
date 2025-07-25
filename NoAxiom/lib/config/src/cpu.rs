//! architecture and hardware configs

/// CPU count for multi-core
#[cfg(feature = "multicore")]
pub const CPU_NUM: usize = 2;

/// CPU count, always be 1 since multicore is off
#[cfg(not(feature = "multicore"))]
pub const CPU_NUM: usize = 1;

pub const PLIC_SLOTS: usize = CPU_NUM * 32;

//! architecture and hardware configs

/// CPU count for multi-core
#[cfg(feature = "multicore")]
pub const CPU_NUM: usize = config::arch::CPU_NUM;

/// CPU count, always be 1 since multicore is off
#[cfg(not(feature = "multicore"))]
pub const CPU_NUM: usize = 1;
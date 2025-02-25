//! architecture and hardware configs

/// CPU count for multi-core
#[cfg(feature = "multicore")]
pub const CPU_NUM: usize = 2;

/// CPU count, always be 1 since multicore is off
#[cfg(not(feature = "multicore"))]
pub const CPU_NUM: usize = 1;

pub const FULL_HART_MASK: usize = (1 << CPU_NUM) - 1;
//! architecture and hardware configs

/// CPU count for multi-core
pub const CPU_NUM: usize = 4;

pub const PLIC_SLOTS: usize = CPU_NUM * 32;

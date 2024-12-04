//! time constants

#![allow(unused)]

/// millisecond per second, 1e-3 s
pub const MSEC_PER_SEC: usize = 1000;

/// microsecond per second, 1e-6 s
pub const USEC_PER_SEC: usize = 100_0000;

/// nanosecond per second, 1e-9 s
pub const NSEC_PER_SEC: usize = 10_0000_0000;

/// clock frequency: 4MHz, ns per tick: 250ns
#[cfg(feature = "vf2")]
pub const CLOCK_FREQ: usize = 4000000;

/// clock frequency: 12.5MHz, ns per tick: 80ns
#[cfg(feature = "riscv_qemu")]
pub const CLOCK_FREQ: usize = 12500000;

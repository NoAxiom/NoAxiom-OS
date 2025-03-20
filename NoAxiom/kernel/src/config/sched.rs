//! task schedule configs

use crate::constant::time::*;

/// time slice: 10ms,
/// every second will be spiltted into 100 slices,
pub const TIME_SLICE_PER_SEC: usize = 100;

/// time slice in clock ticks: 125000 ticks
pub const TIME_SLICE_TICKS: usize = CLOCK_FREQ / TIME_SLICE_PER_SEC;

/// load_balance: the overload / underload threshold
/// when `load < average * (1 - 1 / threshold)`, it's underload
/// vice versa
/// WARNING: currently discarded
#[allow(unused)]
pub const LOAD_BALANCE_LIMIT: usize = 3;

/// load balance span
pub const LOAD_BALANCE_TICKS: usize = TIME_SLICE_TICKS * 10;

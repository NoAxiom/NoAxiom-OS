//! task schedule configs

use crate::constant::time::*;

/// this is for multi-level feedback queue schedule algorithm.
/// TODO: maybe should discard?
pub const MLFQ_LEVELS: usize = 32;

/// time slice: 10ms,
/// every second will be spiltted into 100 slices,
pub const TIME_SLICE_PER_SEC: usize = 100;

/// time slice in clock ticks: 125000 ticks
pub const TIME_SLICE_TICKS: usize = CLOCK_FREQ / TIME_SLICE_PER_SEC;

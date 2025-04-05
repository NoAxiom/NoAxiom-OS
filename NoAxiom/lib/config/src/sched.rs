//! task schedule configs

/// time slice: 10ms,
/// every second will be spiltted into 100 slices,
pub const TIME_SLICE_PER_SEC: usize = 100;

/// load_balance: the overload / underload threshold
/// when `load < average * (1 - 1 / threshold)`, it's underload
/// vice versa
/// WARNING: currently discarded
#[allow(unused)]
pub const LOAD_BALANCE_LIMIT: usize = 3;

/// load balance span
pub const LOAD_BALANCE_SLICE_NUM: usize = 10;

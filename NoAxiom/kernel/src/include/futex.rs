pub const FUTEX_WAIT: usize = 0;
pub const FUTEX_WAKE: usize = 1;
pub const FUTEX_FD: usize = 2;
pub const FUTEX_REQUEUE: usize = 3;
pub const FUTEX_CMP_REQUEUE: usize = 4;
pub const FUTEX_WAKE_OP: usize = 5;
pub const FUTEX_LOCK_PI: usize = 6;
pub const FUTEX_UNLOCK_PI: usize = 7;
pub const FUTEX_TRYLOCK_PI: usize = 8;
pub const FUTEX_WAIT_BITSET: usize = 9;
pub const FUTEX_WAKE_BITSET: usize = 10;
pub const FUTEX_WAIT_REQUEUE_PI: usize = 11;
pub const FUTEX_CMP_REQUEUE_PI: usize = 12;
pub const FUTEX_LOCK_PI2: usize = 13;

pub const FUTEX_PRIVATE_FLAG: usize = 128;
pub const FUTEX_CLOCK_REALTIME: usize = 256;
pub const FUTEX_CMD_MASK: usize = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

pub const FUTEX_BITSET_MATCH_ANY: u32 = u32::MAX;

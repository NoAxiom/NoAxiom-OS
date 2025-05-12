pub const FUTEX_WAIT: usize = 0;
pub const FUTEX_WAKE: usize = 1;
pub const FUTEX_REQUEUE: usize = 3;

pub const FUTEX_PRIVATE_FLAG: usize = 128;
pub const FUTEX_CLOCK_REALTIME: usize = 256;
pub const FUTEX_CMD_MASK: usize = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

pub const FD_SET_SIZE: usize = 1024;
pub const FD_SIZE: usize = 8 * core::mem::size_of::<u64>();
pub const FD_SET_LEN: usize = FD_SET_SIZE / FD_SIZE;

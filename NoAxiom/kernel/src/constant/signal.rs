pub const MAX_SIGNUM: u32 = 64;

// The SIG_DFL and SIG_IGN macros expand into integral expressions that are not
// equal to an address of any function. The macros define signal handling
// strategies for signal() function.
pub const SIG_DFL: usize = 0; // default signal handling
pub const SIG_IGN: usize = 1; // signal is ignored
use core::result::Result;

use crate::include::result::Errno;

pub mod fs;
pub mod io;
pub mod mm;
pub mod net;
pub mod others;
pub mod process;
pub mod sched;
pub mod signal;
pub mod syscall;
pub mod system;
pub mod time;
pub mod utils;

#[macro_use]
pub mod macros;

pub use syscall::Syscall;

pub type SysResult<T> = Result<T, Errno>;
pub type SyscallResult = SysResult<isize>;

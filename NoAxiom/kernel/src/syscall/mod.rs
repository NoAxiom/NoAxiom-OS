use core::result::Result;

use crate::{constant::register::*, include::result::Errno};

pub mod fs;
pub mod mm;
pub mod others;
pub mod process;
pub mod syscall;
pub mod sys_args;

#[macro_use]
pub mod macros;

pub use syscall::{syscall, Syscall};

pub type SysResult<T> = Result<T, Errno>;
pub type SyscallResult = SysResult<isize>;

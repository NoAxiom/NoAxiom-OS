use core::result::Result;

use crate::{constant::register::*, nix::result::Errno};

pub mod fs;
pub mod mm;
pub mod others;
pub mod process;
pub mod syscall;

pub use syscall::{syscall, Syscall};

pub type SyscallResult = Result<isize, Errno>;

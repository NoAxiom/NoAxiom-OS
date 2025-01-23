use alloc::{sync::Arc, vec::Vec};

use super::{
    stdio::{Stdin, Stdout},
    vfs::basic::file::File,
};
use crate::{
    constant::fs::{RLIMIT_HARD_MAX, RLIMIT_SOFT_MAX},
    nix::result::Errno,
    syscall::{Syscall, SyscallResult},
};

/// Resource Limit from linux
#[derive(Debug, Clone, Copy)]
pub struct RLimit {
    /// Soft limit
    pub rlim_cur: usize,
    /// Hard limit (ceiling for rlim_cur)
    pub rlim_max: usize,
}

#[derive(Clone)]
pub struct FdTable {
    pub table: Vec<Option<Arc<dyn File>>>,
    rlimt: RLimit,
}

impl FdTable {
    pub fn new() -> Self {
        Self {
            table: vec![
                Some(Arc::new(Stdin)),
                Some(Arc::new(Stdout)),
                Some(Arc::new(Stdout)),
            ],
            rlimt: RLimit {
                rlim_cur: RLIMIT_SOFT_MAX,
                rlim_max: RLIMIT_HARD_MAX,
            },
        }
    }

    /// get the soft rlimit of the fd table
    fn rslimit(&self) -> usize {
        self.rlimt.rlim_cur
    }

    /// Allocate a new fd slot
    pub fn alloc_fd(&mut self) -> SyscallResult {
        if let Some(fd) = self.table.iter().position(|x| x.is_none()) {
            return Ok(fd as isize);
        }

        if self.table.len() >= self.rslimit() {
            Err(Errno::EMFILE)
        } else {
            self.table.push(None);
            Ok((self.table.len() - 1) as isize)
        }
    }

    /// Get the `fd` slot, None if `fd` > `table.len()`
    pub fn get(&self, fd: usize) -> Option<Arc<dyn File>> {
        if fd < self.table.len() {
            self.table[fd].clone()
        } else {
            None
        }
    }

    /// Fill the `fd` slot with None
    pub fn fill_to(&mut self, fd: usize) -> SyscallResult {
        if fd > self.rslimit() {
            return Err(Errno::EBADF);
        }
        for _ in self.table.len()..fd {
            self.table.push(None);
        }
        Ok(fd as isize)
    }

    /// Copy the file descriptor from `old_fd` to `new_fd`
    pub fn copyfrom(&mut self, old_fd: usize, new_fd: usize) -> SyscallResult {
        self.table[new_fd as usize] = self.table[old_fd].clone();
        Ok(new_fd as isize)
    }
}

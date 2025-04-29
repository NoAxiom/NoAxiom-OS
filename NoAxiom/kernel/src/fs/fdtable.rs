use alloc::{sync::Arc, vec::Vec};

use super::vfs::{basic::file::File, TTYFILE};
use crate::{
    constant::fs::{RLIMIT_HARD_MAX, RLIMIT_SOFT_MAX},
    include::{fs::FcntlArgFlags, result::Errno},
    net::socketfile::SocketFile,
    syscall::{SysResult, SyscallResult},
};

/// Resource Limit from linux
#[derive(Debug, Clone, Copy)]
pub struct RLimit {
    /// Soft limit
    pub rlim_cur: usize,
    /// Hard limit (ceiling for rlim_cur)
    pub rlim_max: usize,
}

impl Default for RLimit {
    fn default() -> Self {
        Self {
            rlim_cur: RLIMIT_SOFT_MAX,
            rlim_max: RLIMIT_HARD_MAX,
        }
    }
}

#[derive(Clone)]
pub struct FdTableEntry {
    flags: FcntlArgFlags,
    pub file: Arc<dyn File>,
}

impl FdTableEntry {
    fn tty_file() -> Arc<dyn File> {
        TTYFILE.get().unwrap().clone()
    }
    pub fn std_in() -> Self {
        Self {
            flags: FcntlArgFlags::empty(),
            file: Self::tty_file(),
        }
    }
    pub fn std_out() -> Self {
        Self {
            flags: FcntlArgFlags::empty(),
            file: Self::tty_file(),
        }
    }
    pub fn std_err() -> Self {
        Self::std_out()
    }
    pub fn from_file(file: Arc<dyn File>) -> Self {
        let file_flag = file.flags();
        Self {
            flags: FcntlArgFlags::from_arg(file_flag.clone()),
            file: file.clone(),
        }
    }
}

#[derive(Clone)]
pub struct FdTable {
    pub table: Vec<Option<FdTableEntry>>,
    rlimt: RLimit,
}

impl FdTable {
    pub fn new() -> Self {
        Self {
            table: vec![
                Some(FdTableEntry::std_in()),
                Some(FdTableEntry::std_out()),
                Some(FdTableEntry::std_err()),
            ],
            rlimt: RLimit::default(),
        }
    }

    pub fn rlimit(&self) -> &RLimit {
        &self.rlimt
    }

    pub fn rlimit_mut(&mut self) -> &mut RLimit {
        &mut self.rlimt
    }

    /// get the soft rlimit of the fd table
    fn rslimit(&self) -> usize {
        self.rlimt.rlim_cur
    }

    /// Allocate a new fd slot, if has empty slot, return the first empty slot,
    /// you should use tht fd after alloc immediately
    pub fn alloc_fd(&mut self) -> SysResult<usize> {
        if let Some(fd) = self.table.iter().position(|x| x.is_none()) {
            return Ok(fd);
        }

        if self.table.len() >= self.rslimit() {
            Err(Errno::EMFILE)
        } else {
            self.table.push(None);
            Ok(self.table.len() - 1)
        }
    }

    /// Allocate a new fd slot greater than `fd`, if has empty slot, return the
    /// first empty slot, you should use tht fd after alloc immediately
    pub fn alloc_fd_after(&mut self, fd: usize) -> SysResult<usize> {
        if let Some(fd) = self.table[fd + 1..].iter().position(|x| x.is_none()) {
            return Ok(fd);
        }

        if self.table.len() >= self.rslimit() {
            Err(Errno::EMFILE)
        } else {
            self.fill_to(fd + 1)?;
            Ok(fd + 1)
        }
    }

    /// Get the `fd` slot, None if `fd` > `table.len()`
    pub fn get(&self, fd: usize) -> Option<Arc<dyn File>> {
        if fd < self.table.len() {
            if let Some(entry) = &self.table[fd] {
                return Some(entry.file.clone());
            }
        }
        None
    }

    /// Get the `fd` socket slot, None if `fd` > `table.len()`
    pub fn get_socketfile(&self, fd: usize) -> Option<Arc<SocketFile>> {
        if fd < self.table.len() {
            if let Some(entry) = &self.table[fd] {
                let socket_file = entry.file.clone();
                let socket = socket_file.downcast_arc::<SocketFile>();
                if let Ok(socket) = socket {
                    return Some(socket);
                }
            }
        }

        None
    }

    /// Set the `fd` slot
    pub fn set(&mut self, fd: usize, file: Arc<dyn File>) {
        self.table[fd] = Some(FdTableEntry::from_file(file));
    }

    /// Get the `flags` of the `fd` slot
    pub fn get_fdflag(&self, fd: usize) -> Option<FcntlArgFlags> {
        if fd < self.table.len() {
            if let Some(entry) = &self.table[fd] {
                return Some(entry.flags.clone());
            }
        }
        None
    }

    /// Set the `flags` of the `fd` slot
    pub fn set_fdflag(&mut self, fd: usize, flags: FcntlArgFlags) {
        if fd < self.table.len() {
            if let Some(entry) = &mut self.table[fd] {
                entry.flags = flags;
            }
        }
    }

    /// Fill the `fd` slot with None
    pub fn fill_to(&mut self, fd: usize) -> SyscallResult {
        if fd > self.rslimit() {
            return Err(Errno::EBADF);
        }
        for _ in self.table.len()..fd + 1 {
            self.table.push(None);
        }
        Ok(fd as isize)
    }

    /// Copy the file descriptor from `old_fd` to `new_fd`
    pub fn copyfrom(&mut self, old_fd: usize, new_fd: usize) -> SyscallResult {
        // self.fill_to(core::cmp::max(old_fd, new_fd))?;
        self.table[new_fd] = self.table[old_fd].clone();
        Ok(new_fd as isize)
    }

    pub fn close(&mut self, fd: usize) -> SyscallResult {
        if fd < 3 {
            return Ok(0);
        }
        if fd >= self.table.len() {
            return Err(Errno::EBADF);
        }
        self.table[fd] = None;
        Ok(0)
    }

    pub fn close_on_exec(&mut self) {
        for table_entry in self.table.iter_mut() {
            if let Some(entry) = table_entry {
                if entry.flags.contains(FcntlArgFlags::FD_CLOEXEC) {
                    *table_entry = None;
                }
            }
        }
    }
}

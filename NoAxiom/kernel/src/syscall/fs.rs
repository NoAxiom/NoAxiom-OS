use alloc::vec::Vec;

use super::{Syscall, SyscallResult};
use crate::{
    constant::fs::{AT_FDCWD, STD_ERR, STD_IN, STD_OUT},
    fs::{path::Path, pipe::PipeFile, vfs::root_dentry},
    mm::user_ptr::UserPtr,
    include::{
        fs::{FileFlags, InodeMode},
        result::Errno,
    },
    utils::get_string_from_ptr,
};

impl Syscall<'_> {
    /// Get current working directory
    pub fn sys_getcwd(&self, buf: *mut u8, size: usize) -> SyscallResult {
        info!("[sys_getcwd] buf: {:?}, size: {}", buf, size);
        if buf as usize == 0 {
            return Err(Errno::EFAULT);
        }
        if buf as usize != 0 && size == 0 {
            return Err(Errno::EINVAL);
        }

        let pcb_gaurd = self.task.pcb();
        let cwd = pcb_gaurd.cwd.clone();

        // fixme: size??
        // todo : encapsulate the following write steps
        // let ptr = UserPtr::<&[u8]>::new(buf as usize);
        // let data_cloned = ptr.as_ref_mut(); // this might trigger pagefault
        // *data_cloned = cwd.as_string().as_bytes();

        let buf_slice: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, size) };
        let binding = cwd.as_string();
        let cwd_bytes = binding.as_bytes();
        buf_slice[..cwd_bytes.len()].copy_from_slice(cwd_bytes);

        Ok(buf as isize)
    }

    /// Create a pipe
    pub fn sys_pipe2(&self, pipe: *mut i32, flag: usize) -> SyscallResult {
        let (read_end, write_end) = PipeFile::new_pipe();

        let mut fd_table = self.task.fd_table();
        let read_fd = fd_table.alloc_fd()?;
        let write_fd = fd_table.alloc_fd()?;
        fd_table.set(read_fd as usize, read_end);
        fd_table.set(write_fd as usize, write_end);

        let buf = unsafe { core::slice::from_raw_parts_mut(pipe, 2 * core::mem::size_of::<i32>()) };
        buf[0] = read_fd as i32;
        buf[1] = write_fd as i32;
        info!("[sys_pipe]: read fd {}, write fd {}", read_fd, write_fd);
        Ok(0)
    }

    /// Duplicate a file descriptor
    pub fn sys_dup(&self, fd: usize) -> SyscallResult {
        info!("[sys_dup] fd: {}", fd);

        let mut fd_table = self.task.fd_table();
        if fd_table.get(fd).is_none() {
            return Err(Errno::EBADF);
        }

        let new_fd = fd_table.alloc_fd()?;
        fd_table.copyfrom(fd, new_fd as usize)
    }

    // Duplicate a file descriptor to a specific fd
    pub fn sys_dup3(&self, old_fd: usize, new_fd: usize) -> SyscallResult {
        info!("[sys_dup3] old_fd: {}, new_fd: {}", old_fd, new_fd);

        let mut fd_table = self.task.fd_table();
        if fd_table.get(old_fd).is_none() {
            return Err(Errno::EBADF);
        }

        fd_table.fill_to(new_fd);
        fd_table.copyfrom(old_fd, new_fd as usize)
    }

    /// Switch to a new work directory
    pub fn sys_chdir(&self, path: usize) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(path);
        let path = get_string_from_ptr(&ptr);
        info!("[sys_chdir] path: {}", path);

        let split_path = path.split('/').collect::<Vec<&str>>();
        root_dentry().find_path(&split_path)?;

        let cwd = self.task.pcb().cwd.clone().from_cd(&"..");
        debug!("[sys_chdir] cwd: {:?}", cwd);

        let mut pcb_guard = self.task.pcb();
        pcb_guard.cwd = cwd.from_cd(&path);
        Ok(0)
    }

    /// Open or create a file
    pub fn sys_openat(&self, fd: isize, filename: usize, flags: u32, mode: u32) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(filename);
        let path = get_string_from_ptr(&ptr);
        info!(
            "[sys_openat] dirfd {}, flags {:?}, filename {}, mode {}",
            fd, flags, path, mode
        );

        let cwd = self.task.pcb().cwd.clone();
        let mut fd_table = self.task.fd_table();
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        let mode = InodeMode::from_bits_truncate(mode);
        let path = if fd == AT_FDCWD {
            cwd.from_cd(&path)
        } else {
            cwd
        };

        // todo: CHECK flags!

        let dentry = path.dentry();

        let file = dentry.open()?;
        file.set_flags(flags);
        let fd = fd_table.alloc_fd()?;
        fd_table.set(fd as usize, file);

        Ok(fd)
    }

    /// Read data from a file descriptor
    ///
    /// Return val:
    /// 1. len > buf.size: ???
    /// 2. len <= buf.size:
    ///     - len > file_remain_size: file_remain_size
    ///     - len <= file_remain_size: len
    ///     - file_remain_size == 0: 0, which means EOF
    /// 3. fd is closed: -1
    pub async fn sys_read(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        info!("[sys_read] fd: {}, buf: {:?}, len: {}", fd, buf, len);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        // todo: INTERRUPT_BY_SIGNAL FUTURE

        if not_stdio(fd) && !file.meta().readable() {
            return Err(Errno::EINVAL);
        }

        // todo: check lazy?
        // check_mut_slice(buf as *mut u8, len);
        let buf_slice: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let content = file.read_all().await?;
        buf_slice.copy_from_slice(&content[..len]);

        // todo: len or readsize?
        Ok(len as isize)
    }

    /// Write data to a file descriptor
    pub async fn sys_write(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        trace!("[sys_write] fd: {}, buf: {:?}, len: {}", fd, buf, len);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        if not_stdio(fd) && !file.meta().writable() {
            return Err(Errno::EINVAL);
        }

        // check_mut_slice(buf as *mut u8, len);
        let buf_slice: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let buf = buf_slice.to_vec();
        file.write_at(0, &buf).await?;

        // todo: len or writesize?
        Ok(len as isize)
    }

    /// Create a directory
    pub async fn sys_mkdirat(&self, dirfd: isize, path: usize, mode: u32) -> SyscallResult {
        let mode = InodeMode::from_bits_truncate(mode);
        let ptr = UserPtr::<u8>::new(path);
        let path_str = get_string_from_ptr(&ptr);
        let path = if !path_str.starts_with('/') {
            if dirfd == AT_FDCWD {
                let cwd = self.task.pcb().cwd.clone().from_cd(&"..");
                debug!("[sys_mkdirat] cwd: {:?}", cwd);
                cwd.from_cd_or_create(&path_str)
            } else {
                todo!()
            }
        } else {
            Path::from(path_str)
        };

        info!(
            "[sys_mkdirat] dirfd: {}, path: {:?}, mode: {:?}",
            dirfd, path, mode
        );

        let dentry = path.dentry();
        if dentry.inode().is_ok() {
            return Err(Errno::EEXIST);
        }

        let parent = dentry.parent().unwrap();
        parent
            .add_dir_child(&dentry.name(), &mode.union(InodeMode::DIR))
            .await?;
        Ok(0)
    }

    pub fn sys_close(&self, fd: usize) -> SyscallResult {
        info!("[sys_close] fd: {}", fd);
        let mut fd_table = self.task.fd_table();
        fd_table.close(fd)
    }

    pub fn empty(&self) -> SyscallResult {
        Ok(0)
    }
}

fn not_stdio(fd: usize) -> bool {
    fd != STD_IN && fd != STD_OUT && fd != STD_ERR
}

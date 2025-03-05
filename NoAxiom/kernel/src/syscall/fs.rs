use alloc::vec::Vec;

use super::{Syscall, SyscallResult};
use crate::{
    constant::fs::{AT_FDCWD, STD_ERR, STD_IN, STD_OUT},
    fs::{path::Path, pipe::PipeFile, vfs::root_dentry},
    include::{
        fs::{FileFlags, InodeMode, Kstat},
        result::Errno,
    },
    mm::user_ptr::UserPtr,
    utils::get_string_from_ptr,
};

impl Syscall<'_> {
    /// Get current working directory
    pub async fn sys_getcwd(&self, buf: usize, size: usize) -> SyscallResult {
        info!("[sys_getcwd] buf: {:?}, size: {}", buf, size);
        if buf as usize == 0 {
            return Err(Errno::EFAULT);
        }
        if buf as usize != 0 && size == 0 {
            return Err(Errno::EINVAL);
        }

        let cwd = self.task.pcb().cwd.clone();
        let cwd_str = cwd.as_string();
        let cwd_bytes = cwd_str.as_bytes();

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(size).await?;
        buf_slice[..cwd_bytes.len()].copy_from_slice(cwd_bytes);

        Ok(buf as isize)
    }

    /// Create a pipe
    pub async fn sys_pipe2(&self, pipe: usize, _flag: usize) -> SyscallResult {
        let (read_end, write_end) = PipeFile::new_pipe();

        let mut fd_table = self.task.fd_table();
        let read_fd = fd_table.alloc_fd()?;
        let write_fd = fd_table.alloc_fd()?;
        fd_table.set(read_fd as usize, read_end);
        fd_table.set(write_fd as usize, write_end);

        let user_ptr = UserPtr::<u8>::new(pipe);
        let buf_slice = user_ptr.as_slice_mut_checked(2).await?;
        //? fd as u8 is right?
        buf_slice[0] = read_fd as u8;
        buf_slice[1] = write_fd as u8;
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

        fd_table.fill_to(new_fd)?;
        fd_table.copyfrom(old_fd, new_fd as usize)
    }

    /// Switch to a new work directory
    pub fn sys_chdir(&self, path: usize) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(path);
        let path = get_string_from_ptr(&ptr);
        info!("[sys_chdir] path: {}", path);

        // check if the path is valid
        let split_path = path.split('/').collect::<Vec<&str>>();
        root_dentry().find_path(&split_path)?;

        let cwd = self.task.pcb().cwd.clone().from_cd(&"..");

        let mut pcb_guard = self.task.pcb();
        pcb_guard.cwd = cwd.from_cd(&path);
        Ok(0)
    }

    /// Open or create a file
    pub async fn sys_openat(
        &self,
        fd: isize,
        filename: usize,
        flags: u32,
        mode: u32,
    ) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(filename);
        let path_str = get_string_from_ptr(&ptr);
        info!(
            "[sys_openat] dirfd {}, flags {:#x}, filename {}, mode {}",
            fd, flags, path_str, mode
        );

        let mut fd_table = self.task.fd_table();
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        let mode = InodeMode::from_bits_truncate(mode);
        let path = if !path_str.starts_with('/') {
            if fd == AT_FDCWD {
                let cwd = self.task.pcb().cwd.clone().from_cd(&"..");
                trace!("[sys_openat] cwd: {:?}", cwd);
                cwd.from_cd_or_create(&path_str)
            } else {
                let cwd = fd_table
                    .get(fd as usize)
                    .ok_or(Errno::EBADF)?
                    .dentry()
                    .path();
                trace!("[sys_openat] cwd: {:?}", cwd);
                cwd.from_cd_or_create(&path_str)
            }
        } else {
            Path::from(path_str)
        };

        let dentry = path.dentry();

        if flags.contains(FileFlags::O_CREATE) {
            if flags.contains(FileFlags::O_EXCL) && !dentry.is_negetive() {
                return Err(Errno::EEXIST);
            }
            let parent = dentry.parent().unwrap();
            parent
                .create(&dentry.name(), InodeMode::FILE | mode)
                .await?;
        }

        let inode = dentry.inode()?;
        if flags.contains(FileFlags::O_DIRECTORY) && !inode.file_type() == InodeMode::DIR {
            return Err(Errno::ENOTDIR);
        }

        let file = dentry.open()?;
        let file_path = file.dentry().path();
        file.set_flags(flags);
        let fd = fd_table.alloc_fd()?;
        fd_table.set(fd as usize, file);

        trace!("[sys_openat] succeed fd: {}, path: {:?}", fd, file_path);
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

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

        if file.is_stdio() {
            let read_size = file.base_read(0, buf_slice).await?;
            return Ok(read_size as isize);
        }

        if !file.meta().readable() {
            return Err(Errno::EINVAL);
        }

        let read_size = file.read(buf_slice).await?;

        Ok(read_size as isize)
    }

    /// Write data to a file descriptor
    pub async fn sys_write(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        trace!("[sys_write] fd: {}, buf: {:?}, len: {}", fd, buf, len);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

        if file.is_stdio() {
            let write_size = file.base_write(0, buf_slice).await?;
            return Ok(write_size as isize);
        }

        if !file.meta().writable() {
            return Err(Errno::EINVAL);
        }

        let write_size = file.write(buf_slice).await?;

        Ok(write_size as isize)
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

    pub fn sys_fstat(&self, fd: usize, stat_buf: usize) -> SyscallResult {
        debug!("[sys_fstat]: fd: {}, stat_buf: {:#x}", fd, stat_buf);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        let kstat = Kstat::from_stat(file.inode().stat()?);
        let ptr = UserPtr::<Kstat>::new(stat_buf as usize);
        ptr.write_volatile(kstat);
        Ok(0)
    }
}

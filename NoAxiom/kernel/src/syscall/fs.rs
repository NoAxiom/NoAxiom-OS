use alloc::{sync::Arc, vec::Vec};

use config::task::INIT_PROCESS_ID;
use ksync::mutex::check_no_lock;

use super::{SysResult, Syscall, SyscallResult};
use crate::{
    constant::fs::AT_FDCWD,
    fs::{fdtable::RLimit, manager::FS_MANAGER, path::Path, pipe::PipeFile, vfs::root_dentry},
    include::{
        fs::{FcntlArgFlags, FcntlFlags, FileFlags, InodeMode, Iovec, Kstat, MountFlags},
        resource::Resource,
        result::Errno,
    },
    mm::user_ptr::UserPtr,
    task::Task,
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

        let cwd = self.task.cwd().clone();
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

        let user_ptr = UserPtr::<i32>::new(pipe);
        let buf_slice = user_ptr.as_slice_mut_checked(2).await?;

        let mut fd_table = self.task.fd_table();
        let read_fd = fd_table.alloc_fd()?;
        fd_table.set(read_fd, read_end);
        buf_slice[0] = read_fd as i32;

        let write_fd = fd_table.alloc_fd()?;
        fd_table.set(write_fd, write_end);
        buf_slice[1] = write_fd as i32;

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
        fd_table.copyfrom(fd, new_fd)
    }

    // Duplicate a file descriptor to a specific fd
    pub fn sys_dup3(&self, old_fd: usize, new_fd: usize) -> SyscallResult {
        info!("[sys_dup3] old_fd: {}, new_fd: {}", old_fd, new_fd);

        let mut fd_table = self.task.fd_table();
        if fd_table.get(old_fd).is_none() {
            return Err(Errno::EBADF);
        }

        fd_table.fill_to(new_fd)?;
        fd_table.copyfrom(old_fd, new_fd)
    }

    /// Switch to a new work directory
    pub fn sys_chdir(&self, path: usize) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(path);
        let path = get_string_from_ptr(&ptr);
        info!("[sys_chdir] path: {}", path);

        // check if the path is valid
        let split_path = path.split('/').collect::<Vec<&str>>();
        root_dentry().find_path(&split_path)?;

        let mut cwd_guard = self.task.cwd();
        *cwd_guard = cwd_guard.clone().from_cd(&"..")?.from_cd(&path)?;
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
        let mode = InodeMode::from_bits_truncate(mode);
        info!(
            "[sys_openat] dirfd {}, flags {:#x}, filename {}, mode {:?}",
            fd, flags, path_str, mode
        );

        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        let split_path = path_str.split('/').collect::<Vec<&str>>();
        let target = root_dentry().find_path(&split_path);

        if flags.contains(FileFlags::O_CREATE) {
            info!("[sys_openat] O_CREATE");
            // check if the file already exists
            if let Ok(dentry) = target {
                if flags.contains(FileFlags::O_EXCL) && !dentry.is_negetive() {
                    return Err(Errno::EEXIST);
                }
            }
        } else {
            target?;
        }

        // fixme: now if has O_CREATE flag, and the file already exists, we just open it
        let path = get_path_or_create(
            self.task.clone(),
            filename,
            fd as isize,
            mode.union(InodeMode::FILE),
            "sys_openat",
        )
        .await?;
        let dentry = path.dentry();
        let inode = dentry.inode()?;
        if flags.contains(FileFlags::O_DIRECTORY) && !inode.file_type() == InodeMode::DIR {
            return Err(Errno::ENOTDIR);
        }

        let file = dentry.open()?;
        let file_path = file.dentry().path();
        file.set_flags(flags);
        let mut fd_table = self.task.fd_table();
        let fd = fd_table.alloc_fd()?;
        fd_table.set(fd, file);

        trace!("[sys_openat] succeed fd: {}, path: {:?}", fd, file_path);
        Ok(fd as isize)
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

        file.read(buf_slice).await
    }

    /// Read the file associated with the file descriptor fd to iovcnt buffers
    /// of data described by iov
    pub async fn sys_readv(&self, fd: usize, iovp: usize, iovcnt: usize) -> SyscallResult {
        info!(
            "[sys_readv] fd: {}, iovp: {:#x}, iovcnt: {}",
            fd, iovp, iovcnt
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut read_size = 0;
        for i in 0..iovcnt {
            let iov_ptr = UserPtr::<Iovec>::new(iovp + i * Iovec::size());
            // todo: check lazy?
            iov_ptr.as_slice_mut_checked(Iovec::size()).await?;

            let iov = iov_ptr.read();
            let buf_ptr = UserPtr::<u8>::new(iov.iov_base);
            let buf_slice = buf_ptr.as_slice_mut_checked(iov.iov_len).await?;
            read_size += file.read(buf_slice).await?;
        }
        Ok(read_size)
    }

    /// Write data to a file descriptor
    pub async fn sys_write(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        info!("[sys_write] fd: {}, buf: {:?}, len: {}", fd, buf, len);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

        // debug!(
        //     "[sys_write] buf as string: {}",
        //     String::from_utf8_lossy(buf_slice)
        // );

        if file.is_stdio() {
            let write_size = file.base_write(0, buf_slice).await?;
            return Ok(write_size as isize);
        }

        if !file.meta().writable() {
            return Err(Errno::EINVAL);
        }

        file.write(buf_slice).await
    }

    /// Write iovcnt buffers of data described by iov to the file associated
    /// with the file descriptor fd
    pub async fn sys_writev(&self, fd: usize, iovp: usize, iovcnt: usize) -> SyscallResult {
        info!(
            "[sys_writev] fd: {}, iovp: {:#x}, iovcnt: {}",
            fd, iovp, iovcnt
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);

        let mut write_size = 0;
        for i in 0..iovcnt {
            let iov_ptr = UserPtr::<Iovec>::new(iovp + i * Iovec::size());
            // todo: check lazy?
            iov_ptr.as_slice_mut_checked(Iovec::size()).await?;

            let iov = iov_ptr.read();
            let buf_ptr = UserPtr::<u8>::new(iov.iov_base);
            let buf_slice = buf_ptr.as_slice_mut_checked(iov.iov_len).await?;
            write_size += file.write(buf_slice).await?;
        }
        Ok(write_size)
    }

    /// Create a directory
    pub async fn sys_mkdirat(&self, dirfd: isize, path: usize, mode: u32) -> SyscallResult {
        let mode = InodeMode::from_bits_truncate(mode);
        let path = get_path_or_create(
            self.task.clone(),
            path,
            dirfd,
            mode.union(InodeMode::DIR),
            "sys_mkdirat",
        )
        .await?;
        info!(
            "[sys_mkdirat] dirfd: {}, path: {:?}, mode: {:?}",
            dirfd, path, mode
        );
        Ok(0)
    }

    /// Close a file
    pub fn sys_close(&self, fd: usize) -> SyscallResult {
        info!("[sys_close] fd: {}", fd);
        let mut fd_table = self.task.fd_table();
        fd_table.close(fd)
    }

    /// Get file status
    pub fn sys_fstat(&self, fd: usize, stat_buf: usize) -> SyscallResult {
        info!("[sys_fstat]: fd: {}, stat_buf: {:#x}", fd, stat_buf);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        let kstat = Kstat::from_stat(file.inode().stat()?);
        let ptr = UserPtr::<Kstat>::new(stat_buf as usize);
        ptr.write(kstat);
        Ok(0)
    }

    /// Get file io control
    pub fn sys_ioctl(&self, fd: usize, request: usize, arg: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        fd_table.get(fd).ok_or(Errno::EBADF)?;

        let arg_ptr = UserPtr::<u8>::new(arg);
        use crate::include::fs::TtyIoctlCmd::{self, *};
        let cmd = TtyIoctlCmd::from_repr(request).unwrap();
        info!(
            "[sys_ioctl]: fd: {}, request: {:#x}, argp: {:#x}, cmd: {:?}",
            fd, request, arg, cmd
        );
        match cmd {
            TCGETS => {}
            TCSETS => {}
            TIOCGPGRP => arg_ptr.write(INIT_PROCESS_ID as u8),
            TIOCSPGRP => {}
            TIOCGWINSZ => arg_ptr.write(0),
            _ => return Err(Errno::EINVAL),
        }
        Ok(0)
    }

    pub async fn sys_newfstat(
        &self,
        dirfd: isize,
        path: usize,
        stat_buf: usize,
        _flags: usize,
    ) -> SyscallResult {
        let path = get_path_or_create(
            self.task.clone(),
            path,
            dirfd,
            InodeMode::empty(),
            "sys_newfstat",
        )
        .await?;
        info!(
            "[sys_newfstat] dirfd: {}, path: {:?}, stat_buf: {:#x}",
            dirfd, path, stat_buf
        );
        let kstat = Kstat::from_stat(path.dentry().inode()?.stat()?);
        let ptr = UserPtr::<Kstat>::new(stat_buf as usize);
        ptr.write(kstat);
        Ok(0)
    }

    /// Get dentries in a directory
    pub async fn sys_getdents64(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        info!("[sys_getdents64] fd: {}, buf: {:#x}, len: {}", fd, buf, len);
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;
        assert!(check_no_lock());
        file.read_dir(buf_slice).await
    }

    /// Mount a file system
    pub async fn sys_mount(
        &self,
        special: usize,
        dir: usize,
        fstype: usize,
        flags: usize,
        _data: usize,
    ) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(special);
        let special = get_string_from_ptr(&ptr);
        let ptr = UserPtr::<u8>::new(dir);
        let dir = get_string_from_ptr(&ptr);
        let ptr = UserPtr::<u8>::new(fstype);
        let fstype = get_string_from_ptr(&ptr);
        let flags = MountFlags::from_bits(flags as u32).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_mount] special: {}, dir: {}, fstype: {}, flags: {:?}",
            special, dir, fstype, flags
        );

        let fs = FS_MANAGER.get(&fstype).ok_or(Errno::EINVAL)?;

        // normally, we should choose the device by special
        // but now we just use the default device
        let device = driver::get_blk_dev();

        let mut split_path = dir.split('/').collect::<Vec<&str>>();
        let name = split_path.pop().unwrap();
        let parent = root_dentry().find_path(&split_path)?;
        trace!("[sys_mount] parent: {:?}, name: {}", parent.name(), name);
        fs.root(Some(parent), flags, name, Some(device)).await;
        Ok(0)
    }

    /// Unmount a file system
    pub fn sys_umount2(&self, dir: usize, flags: usize) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(dir);
        let dir = get_string_from_ptr(&ptr);
        info!("[sys_umount2] target: {}", dir);

        let _flags = MountFlags::from_bits(flags as u32).ok_or(Errno::EINVAL)?;

        let mut split_path = dir.split('/').collect::<Vec<&str>>();
        let name = split_path.pop().unwrap();
        let parent = root_dentry().find_path(&split_path)?;
        parent.remove_child(name).unwrap();

        Ok(0)
    }

    /// Create a hard link
    pub fn sys_linkat(
        &self,
        olddirfd: usize,
        oldpath: usize,
        newdirfd: usize,
        newpath: usize,
        _flags: usize,
    ) -> SyscallResult {
        info!("[sys_linkat]");
        let task = self.task;
        let old_path = get_path(task.clone(), oldpath, olddirfd as isize, "sys_linkat")?;
        let new_path = get_path(task.clone(), newpath, newdirfd as isize, "sys_linkat")?;
        let old_dentry = old_path.dentry();
        let new_dentry = new_path.dentry();
        new_dentry.link_to(old_dentry)?;
        Ok(0)
    }

    pub async fn sys_readlinkat(
        &self,
        dirfd: isize,
        path: usize,
        buf: usize,
        buflen: usize,
    ) -> SyscallResult {
        let path = get_path(self.task.clone(), path, dirfd, "sys_readlinkat")?;
        info!(
            "[sys_readlinkat] dirfd: {}, path: {:?}, buf: {:#x}, bufsize: {}",
            dirfd, path, buf, buflen,
        );
        let dentry = path.dentry();
        if dentry.inode()?.file_type() != InodeMode::LINK {
            return Err(Errno::EINVAL);
        }
        // todo: read, now just do nothing
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;
        let file = dentry.open()?;
        let res = file.base_readlink(buf_slice).await;
        res
    }

    /// Unlink a file, also delete the file if nlink is 0
    pub async fn sys_unlinkat(&self, dirfd: usize, path: usize, _flags: usize) -> SyscallResult {
        info!(
            "[sys_unlinkat] dirfd: {}, path: {}",
            dirfd as isize,
            get_string_from_ptr(&UserPtr::<u8>::new(path))
        );
        let task = self.task;
        let path = get_path(task.clone(), path, dirfd as isize, "sys_unlinkat")?;
        let dentry = path.dentry();
        dentry.unlink().await?;
        Ok(0)
    }

    /// Get and set resource limits
    pub fn sys_prlimit64(
        &self,
        pid: usize,
        resource: u32,
        new_limit: usize,
        old_limit: usize,
    ) -> SyscallResult {
        let task = if pid == 0 {
            self.task.clone()
        } else if let Some(task) = crate::task::manager::TASK_MANAGER.get(pid) {
            task
        } else {
            Err(Errno::ESRCH)?
        };

        let mut fd_table = task.fd_table();
        let resource = Resource::from_u32(resource)?;
        let new_limit = UserPtr::<RLimit>::new(new_limit);
        let old_limit = UserPtr::<RLimit>::new(old_limit);

        if !old_limit.is_null() {
            old_limit.write(match resource {
                Resource::NOFILE => fd_table.rlimit().clone(),
                Resource::STACK => RLimit::default(),
                _ => todo!(),
            });
        }

        if !new_limit.is_null() {
            info!(
                "[sys_prlimit64] pid: {}, resource: {:?}, new_limit: {:?}",
                pid,
                resource,
                new_limit.read()
            );
            // todo: check before read??
            *fd_table.rlimit_mut() = new_limit.read();
        }
        info!("[sys_prlimit64] pid: {}, resource: {:?}", pid, resource);
        Ok(0)
    }

    /// Manipulate file descriptor. It performs one of the operations described
    /// below on the open file descriptor fd.
    pub fn sys_fcntl(&self, fd: usize, cmd: usize, arg: usize) -> SyscallResult {
        info!("[sys_fcntl] fd: {fd}, cmd: {cmd:?}, arg: {arg}");
        let task = self.task;
        let flags = FileFlags::from_bits_retain(arg as u32);
        let mut fd_table = task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;

        let op = FcntlFlags::from_bits(cmd).unwrap();
        match op {
            FcntlFlags::F_SETFL => {
                file.set_flags(flags);
                Ok(0)
            }
            FcntlFlags::F_SETFD => {
                let arg = FileFlags::from_bits_retain(arg as u32);
                let fd_flags = FcntlArgFlags::from_arg(arg);
                fd_table.set_fdflag(fd, fd_flags);
                Ok(0)
            }
            FcntlFlags::F_GETFD => {
                let fd_flags = fd_table.get_fdflag(fd).ok_or(Errno::EBADF)?;
                Ok(fd_flags.bits() as isize)
            }
            FcntlFlags::F_GETFL => {
                let file_flag = file.flags();
                Ok(file_flag.bits() as isize)
            }
            FcntlFlags::F_DUPFD => {
                let new_fd = fd_table.alloc_fd_after(fd)?;
                assert!(new_fd > fd);
                fd_table.copyfrom(fd, new_fd)
            }
            FcntlFlags::F_DUPFD_CLOEXEC => {
                let new_fd = fd_table.alloc_fd_after(fd)?;
                assert!(new_fd > fd);
                fd_table.set_fdflag(new_fd, FcntlArgFlags::FD_CLOEXEC);
                fd_table.copyfrom(fd, new_fd)
            }
            _ => {
                unimplemented!("fcntl cmd: {op:?} not implemented");
            }
        }
    }
}

async fn get_path_or_create(
    task: Arc<Task>,
    rawpath: usize,
    fd: isize,
    mode: InodeMode,
    debug_syscall_name: &str,
) -> SysResult<Path> {
    let path_str = get_string_from_ptr(&UserPtr::<u8>::new(rawpath));

    if !path_str.starts_with('/') {
        if fd == AT_FDCWD {
            let cwd = task.cwd().clone();
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            Ok(cwd.from_cd_or_create(&path_str, mode).await)
        } else {
            let cwd = task
                .fd_table()
                .get(fd as usize)
                .ok_or(Errno::EBADF)?
                .dentry()
                .path();
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            Ok(cwd.from_cd_or_create(&path_str, mode).await)
        }
    } else {
        Path::try_from(path_str)
    }
}

fn get_path(
    task: Arc<Task>,
    rawpath: usize,
    fd: isize,
    debug_syscall_name: &str,
) -> SysResult<Path> {
    let path_str = get_string_from_ptr(&UserPtr::<u8>::new(rawpath));
    if !path_str.starts_with('/') {
        if fd == AT_FDCWD {
            let cwd = task.cwd().clone();
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            cwd.from_cd(&path_str)
        } else {
            let cwd = task
                .fd_table()
                .get(fd as usize)
                .ok_or(Errno::EBADF)?
                .dentry()
                .path();
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            cwd.from_cd(&path_str)
        }
    } else {
        Path::try_from(path_str)
    }
}

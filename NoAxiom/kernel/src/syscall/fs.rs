use alloc::{sync::Arc, vec::Vec};

use config::task::INIT_PROCESS_ID;
use ksync::mutex::check_no_lock;

use super::{SysResult, Syscall, SyscallResult};
use crate::{
    constant::fs::{AT_FDCWD, UTIME_NOW, UTIME_OMIT},
    fs::{fdtable::RLimit, manager::FS_MANAGER, path::Path, pipe::PipeFile, vfs::root_dentry},
    include::{
        fs::{
            FcntlArgFlags, FcntlFlags, FileFlags, InodeMode, IoctlCmd, Iovec, Kstat, MountFlags,
            RenameFlags, RtcIoctlCmd, SeekFrom, Statfs, Statx, TtyIoctlCmd, Whence,
        },
        resource::Resource,
        result::Errno,
        time::TimeSpec,
    },
    mm::user_ptr::UserPtr,
    task::Task,
    time::gettime::get_time_duration,
    utils::get_string_from_ptr,
};

impl Syscall<'_> {
    /// Get current working directory
    pub async fn sys_getcwd(&self, buf: usize, size: usize) -> SyscallResult {
        if buf as usize == 0 {
            return Err(Errno::EFAULT);
        }
        if buf as usize != 0 && size == 0 {
            return Err(Errno::EINVAL);
        }

        let cwd = self.task.cwd().clone();
        let cwd_str = format!("{}\0", cwd.as_str());
        let cwd_bytes = cwd_str.as_bytes();

        info!("[sys_getcwd] buf: {:?}, size: {}, cwd:{:?}", buf, size, cwd);

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

        let mut cwd_guard = self.task.cwd();
        if path.starts_with('/') {
            *cwd_guard = Path::try_from(path)?;
        } else {
            *cwd_guard = cwd_guard.clone().from_cd(&path)?;
        }
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
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_openat] dirfd {}, flags {:?}, filename {}, mode {:?}",
            fd, flags, path_str, mode
        );

        let path = if flags.contains(FileFlags::O_CREATE) {
            info!("[sys_openat] O_CREATE");
            // check if the file already exists, ignore it currently
            // if flags.contains(FileFlags::O_EXCL) {
            //     if get_path(self.task.clone(), filename, fd, "sys_openat").is_ok() {
            //         return Err(Errno::EEXIST);
            //     }
            // }
            get_path_or_create(
                self.task.clone(),
                filename,
                fd,
                mode.union(InodeMode::FILE),
                "sys_openat",
            )
            .await?
        } else {
            get_path(self.task.clone(), filename, fd, "sys_openat")?
        };

        // fixme: now if has O_CREATE flag, and the file already exists, we just open it

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

        info!("[sys_openat] succeed fd: {}, path: {:?}", fd, file_path);
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
        // let file_name = file.dentry().path()?;
        // info!("[sys_read] file_name: {:?}", file_name);

        // todo: INTERRUPT_BY_SIGNAL FUTURE

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

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
        // let file_name = file.dentry().path()?;
        // info!("[sys_readv] file_name: {:?}", file_name);

        let mut read_size = 0;
        for i in 0..iovcnt {
            let iov_ptr = UserPtr::<Iovec>::new(iovp + i * Iovec::size());
            // todo: check lazy?
            iov_ptr.as_slice_mut_checked(Iovec::size()).await?;

            let iov = iov_ptr.read().await?;
            let buf_ptr = UserPtr::<u8>::new(iov.iov_base);
            let buf_slice = buf_ptr.as_slice_mut_checked(iov.iov_len).await?;
            read_size += file.read(buf_slice).await?;
        }
        Ok(read_size)
    }

    pub async fn sys_pread64(
        &self,
        fd: usize,
        buf: usize,
        len: usize,
        offset: usize,
    ) -> SyscallResult {
        info!(
            "[sys_pread64] fd: {}, buf: {:#x}, len: {}, offset: {}",
            fd, buf, len, offset
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_pread64] file_name: {:?}", file_name);
        if !file.meta().readable() {
            return Err(Errno::EINVAL);
        }
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;
        file.read_at(offset, buf_slice).await
    }

    /// Write data to a file descriptor
    pub async fn sys_write(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        info!("[sys_write] fd: {}, buf: {:?}, len: {}", fd, buf, len);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_write] file_name: {:?}", file_name);

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

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
        // let file_name = file.dentry().path()?;
        // info!("[sys_writev] file_name: {:?}", file_name);

        let mut write_size = 0;
        for i in 0..iovcnt {
            let iov_ptr = UserPtr::<Iovec>::new(iovp + i * Iovec::size());
            // todo: check lazy?
            iov_ptr.as_slice_mut_checked(Iovec::size()).await?;

            let iov = iov_ptr.read().await?;
            // let buf_ptr = UserPtr::<u8>::new(iov.iov_base);
            // let buf_slice = buf_ptr.as_slice_mut_checked(iov.iov_len).await?;
            let buf_slice =
                unsafe { core::slice::from_raw_parts(iov.iov_base as *const u8, iov.iov_len) };
            write_size += file.write(buf_slice).await?;
        }
        Ok(write_size)
    }

    pub async fn sys_pwrite64(
        &self,
        fd: usize,
        buf: usize,
        len: usize,
        offset: usize,
    ) -> SyscallResult {
        info!(
            "[sys_pwrite64] fd: {}, buf: {:#x}, len: {}, offset: {}",
            fd, buf, len, offset
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_pwrite64] file_name: {:?}", file_name);
        if !file.meta().writable() {
            return Err(Errno::EINVAL);
        }
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;
        file.write_at(offset, buf_slice).await
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
    pub async fn sys_fstat(&self, fd: usize, stat_buf: usize) -> SyscallResult {
        trace!("[sys_fstat]: fd: {}, stat_buf: {:#x}", fd, stat_buf);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_fstat] file_name: {:?}", file_name);
        let kstat = Kstat::from_stat(file.inode().stat()?);
        let ptr = UserPtr::<Kstat>::new(stat_buf);
        ptr.write(kstat).await?;
        Ok(0)
    }

    pub async fn sys_statx(
        &self,
        dirfd: isize,
        path: usize,
        flags: u32,
        mask: u32,
        buf: usize,
    ) -> SyscallResult {
        let path = get_path(self.task.clone(), path, dirfd, "sys_statx")?;
        let flags = FcntlArgFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_statx] dirfd: {}, path: {:?}, flags: {:?}",
            dirfd, path, flags,
        );
        let statx = path.dentry().inode()?.statx(mask)?;
        let ptr = UserPtr::<Statx>::new(buf);
        ptr.write(statx).await?;
        Ok(0)
    }

    /// Get file status
    pub async fn sys_newfstatat(
        &self,
        dirfd: isize,
        path: usize,
        stat_buf: usize,
        _flags: usize,
    ) -> SyscallResult {
        let path = get_path(self.task.clone(), path, dirfd, "sys_newfstat")?;
        trace!(
            "[sys_newfstat] dirfd: {}, path: {:?}, stat_buf: {:#x}",
            dirfd,
            path,
            stat_buf
        );
        let kstat = Kstat::from_stat(path.dentry().inode()?.stat()?);
        let ptr = UserPtr::<Kstat>::new(stat_buf);
        ptr.write(kstat).await?;
        Ok(0)
    }

    /// Get file io control
    pub async fn sys_ioctl(&self, fd: usize, request: usize, arg: usize) -> SyscallResult {
        info!(
            "[sys_ioctl] fd: {}, request: {:#x}, arg: {:#x}",
            fd, request, arg
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_ioctl] file_name: {:?}", file_name);

        let arg_ptr = UserPtr::<u8>::new(arg);
        let cmd = if let Some(cmd) = TtyIoctlCmd::from_repr(request) {
            IoctlCmd::Tty(cmd)
        } else if let Some(cmd) = RtcIoctlCmd::from_repr(request) {
            IoctlCmd::Rtc(cmd)
        } else {
            return Err(Errno::EINVAL);
        };
        trace!(
            "[sys_ioctl]: fd: {}, request: {:#x}, argp: {:#x}, cmd: {:?}",
            fd,
            request,
            arg,
            cmd
        );
        match cmd {
            IoctlCmd::Tty(x) => match x {
                TtyIoctlCmd::TCGETS => {}
                TtyIoctlCmd::TCSETS => {}
                TtyIoctlCmd::TIOCGPGRP => arg_ptr.write(INIT_PROCESS_ID as u8).await?,
                TtyIoctlCmd::TIOCSPGRP => {}
                TtyIoctlCmd::TIOCGWINSZ => arg_ptr.write(0).await?,
                _ => {
                    error!("[sys_ioctl] request {} is not supported", request);
                    return Err(Errno::EINVAL);
                }
            },
            IoctlCmd::Rtc(x) => match x {
                RtcIoctlCmd::RTCRDTIME => {
                    return file.ioctl(request, arg);
                }
            },
        }
        Ok(0)
    }

    /// Get dentries in a directory
    pub async fn sys_getdents64(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        info!("[sys_getdents64] fd: {}, buf: {:#x}, len: {}", fd, buf, len);
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        // let file_name = file.dentry().path()?;
        // info!("[sys_getdents64] file_name: {:?}", file_name);
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
        let device = crate::fs::blockcache::get_block_cache();

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
        olddirfd: isize,
        oldpath: usize,
        newdirfd: isize,
        newpath: usize,
        _flags: usize,
    ) -> SyscallResult {
        info!("[sys_linkat]");
        let task = self.task;
        let old_path = get_path(task.clone(), oldpath, olddirfd, "sys_linkat")?;
        let new_path = get_path(task.clone(), newpath, newdirfd, "sys_linkat")?;
        let old_dentry = old_path.dentry();
        let new_dentry = new_path.dentry();
        new_dentry.link_to(old_dentry)?;
        Ok(0)
    }

    /// Read link file, error if the file is not a link
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
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;
        let file = dentry.open()?;
        let res = file.base_readlink(buf_slice).await;
        res
    }

    /// Unlink a file, also delete the file if nlink is 0
    pub async fn sys_unlinkat(&self, dirfd: isize, path: usize, _flags: usize) -> SyscallResult {
        info!(
            "[sys_unlinkat] dirfd: {}, path: {}",
            dirfd,
            get_string_from_ptr(&UserPtr::<u8>::new(path))
        );
        let task = self.task;
        let path = get_path(task.clone(), path, dirfd, "sys_unlinkat")?;
        let dentry = path.dentry();
        dentry.unlink().await?;
        debug!("[sys_unlinkat] unlink ok");
        Ok(0)
    }

    /// Get and set resource limits
    pub async fn sys_prlimit64(
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
            old_limit
                .write(match resource {
                    Resource::NOFILE => fd_table.rlimit().clone(),
                    Resource::STACK => RLimit::default(),
                    _ => todo!(),
                })
                .await?;
        }

        if !new_limit.is_null() {
            info!(
                "[sys_prlimit64] pid: {}, resource: {:?}, new_limit: {:?}",
                pid,
                resource,
                new_limit.read().await?,
            );
            // todo: check before read??
            *fd_table.rlimit_mut() = new_limit.read().await?;
        }
        info!(
            "[sys_prlimit64] pid: {}, resource: {:?} new_limit_addr: {:#x}, old_limit_addr: {:#x}",
            pid,
            resource,
            new_limit.addr(),
            old_limit.addr(),
        );
        Ok(0)
    }

    /// Manipulate file descriptor. It performs one of the operations described
    /// below on the open file descriptor fd.
    pub fn sys_fcntl(&self, fd: usize, cmd: usize, arg: usize) -> SyscallResult {
        let task = self.task;
        let flags = FileFlags::from_bits_retain(arg as u32);
        let mut fd_table = task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        // let file_name = file.dentry().path()?;
        // info!("[sys_fcntl] file_name: {:?}", file_name);
        let op = FcntlFlags::from_bits(cmd).unwrap();

        info!("[sys_fcntl] fd: {fd}, cmd: {op:?}, arg: {flags:?}");
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

    /// Copy from input file descriptor to output file descriptor
    pub async fn sys_sendfile(
        &self,
        out_fd: usize,
        in_fd: usize,
        offset: usize,
        count: usize,
    ) -> SyscallResult {
        info!("[sys_sendfile] out_fd: {out_fd}, in_fd: {in_fd}, offset: {offset}, count: {count}");
        let fd_table = self.task.fd_table();
        let out_file = fd_table.get(out_fd).ok_or(Errno::EBADF)?;
        let in_file = fd_table.get(in_fd).ok_or(Errno::EBADF)?;
        info!(
            "[sys_sendfile] out_file: {}, in_file: {}",
            out_file.name(),
            in_file.name()
        );
        drop(fd_table);
        if !out_file.meta().writable() || !in_file.meta().readable() {
            return Err(Errno::EBADF);
        }

        let offset_ptr = UserPtr::<usize>::new(offset);
        let mut buf = vec![0u8; count];
        let read_len = if !offset_ptr.is_null() {
            let offset = offset_ptr.read().await?;
            let read_len = in_file.read_at(offset, &mut buf).await? as usize;
            offset_ptr.write(offset + read_len).await?;
            read_len
        } else {
            in_file.read(&mut buf).await? as usize
        };

        out_file.write(&buf[..read_len]).await
    }

    /// Check if a file exists
    pub async fn sys_faccessat(
        &self,
        dirfd: isize,
        path: usize,
        mode: usize,
        flag: usize,
    ) -> SyscallResult {
        info!(
            "[sys_faccessat] faccessat file: {:?}, flag:{:?}, mode:{:?}, just check path",
            path, flag, mode
        );
        get_path(self.task.clone(), path, dirfd, "sys_faccessat")?;
        Ok(0)
    }

    /// Modify timestamp of a file
    pub async fn sys_utimensat(
        &self,
        dirfd: isize,
        path: usize,
        times: usize,
        flags: usize,
    ) -> SyscallResult {
        let flags = FileFlags::from_bits(flags as u32).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_utimensat] dirfd: {}, times: {:#x}, flags: {:?}",
            dirfd, times, flags
        );
        let fd_table = self.task.fd_table();
        let path_ptr = UserPtr::<u8>::new(path);
        let times_ptr = UserPtr::<TimeSpec>::new(times);
        let inode = if path_ptr.is_null() {
            match dirfd {
                AT_FDCWD => return Err(Errno::EINVAL),
                fd => fd_table.get(fd as usize).ok_or(Errno::EBADF)?.inode(),
            }
        } else {
            let path = get_path(self.task.clone(), path, dirfd, "sys_utimensat")?;
            path.dentry().inode()?
        };

        let current = TimeSpec::from(get_time_duration());
        let (mut atime, mut mtime, ctime) = (Some(current), Some(current), Some(current));
        if !times_ptr.is_null() {
            for i in 0..2 {
                let times_ptr = UserPtr::<TimeSpec>::new(times + i * TimeSpec::size());
                let time_spec = times_ptr.read().await?;
                match time_spec.tv_nsec {
                    UTIME_NOW => {}
                    UTIME_OMIT => {
                        if i == 0 {
                            atime = None;
                        } else {
                            mtime = None;
                        }
                    }
                    _ => {
                        if i == 0 {
                            atime = Some(time_spec);
                        } else {
                            mtime = Some(time_spec);
                        }
                    }
                }
            }
        }

        inode.set_time(&atime, &mtime, &ctime);
        Ok(0)
    }

    /// Seek a file, move the file offset to a new position
    pub fn sys_lseek(&self, fd: usize, offset: isize, whence: usize) -> SyscallResult {
        info!(
            "[sys_lseek] fd: {}, offset: {}, whence: {}",
            fd, offset, whence
        );
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_lseek] file_name: {:?}", file_name);
        let whence = Whence::from_repr(whence).ok_or(Errno::EINVAL)?;

        match whence {
            Whence::SeekSet => file.seek(SeekFrom::Start(offset as u64)),
            Whence::SeekCur => file.seek(SeekFrom::Current(offset as i64)),
            Whence::SeekEnd => file.seek(SeekFrom::End(offset as i64)),
            e => unimplemented!("lseek whence unimplemented: {e:?}"),
        }
    }

    /// Rename a file
    pub async fn sys_renameat2(
        &self,
        old_dirfd: isize,
        old_path: usize,
        new_dirfd: isize,
        new_path: usize,
        flags: i32,
    ) -> SyscallResult {
        let old_path = get_path(self.task.clone(), old_path, old_dirfd, "sys_renameat2")?;
        let new_path = get_path(self.task.clone(), new_path, new_dirfd, "sys_renameat2")?;
        let flags = RenameFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_renameat2] old_path: {:?}, new_path: {:?}",
            old_path, new_path
        );

        let old_dentry = old_path.dentry();
        let new_dentry = new_path.dentry();
        old_dentry.rename_to(new_dentry, flags).await?;
        Ok(0)
    }

    /// Truncate a file to a specified length
    pub async fn sys_ftruncate(&self, fd: usize, length: usize) -> SyscallResult {
        info!("[sys_ftruncate] fd: {}, length: {}", fd, length);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_ftruncate] file_name: {:?}", file_name);
        file.inode().set_size(length);
        file.inode().truncate(length).await?;
        Ok(0)
    }

    /// get fs status
    pub async fn sys_statfs(&self, path: usize, buf: usize) -> SyscallResult {
        let path = get_string_from_ptr(&UserPtr::<u8>::new(path as usize));
        info!("[sys_statfs] path: {}, buf: {:#x}", path, buf);
        let statfs = Statfs::new();
        let ptr = UserPtr::<Statfs>::new(buf);
        ptr.write(statfs).await?;
        Ok(0)
    }

    /// splice data from one file descriptor to another
    pub async fn sys_splice(
        &self,
        fd_in: usize,
        off_in: usize,
        fd_out: usize,
        off_out: usize,
        len: usize,
        _flags: usize,
    ) -> SyscallResult {
        info!(
            "[sys_splice] fd_in: {}, off_in: {:#x}, fd_out: {}, off_out: {:#x}, len: {}",
            fd_in, off_in, fd_out, off_out, len
        );
        let fd_table = self.task.fd_table();
        let file_in = fd_table.get(fd_in).ok_or(Errno::EBADF)?;
        let file_out = fd_table.get(fd_out).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_in_name = file_in.dentry().path()?;
        // let file_out_name = file_out.dentry().path()?;
        // info!(
        //     "[sys_fcntl] file_in_name: {:?}, file_out_name: {:?}",
        //     file_in_name, file_out_name
        // );
        let file_in_type = file_in.inode().file_type();
        let file_out_type = file_out.inode().file_type();
        let off_in = UserPtr::<i64>::new(off_in);
        let off_out = UserPtr::<i64>::new(off_out);

        if file_in_type == InodeMode::FIFO && !off_in.is_null() {
            return Err(Errno::ESPIPE);
        }
        if file_out_type == InodeMode::FIFO && !off_out.is_null() {
            return Err(Errno::ESPIPE);
        }
        if !file_in_type == InodeMode::FIFO && !file_out_type == InodeMode::FIFO {
            return Err(Errno::EINVAL);
        }
        if !file_in.meta().readable() || !file_out.meta().writable() {
            return Err(Errno::EBADF);
        }
        if Arc::ptr_eq(&file_in.inode(), &file_out.inode()) {
            return Err(Errno::EINVAL);
        }

        let mut buf = vec![0; len];
        let in_offset = if !off_in.is_null() {
            let off_in = off_in.read().await?;
            if off_in < 0 {
                return Err(Errno::EINVAL);
            }
            off_in as usize
        } else {
            0
        };
        let in_len = file_in.read_at(in_offset, &mut buf).await?;
        if in_len == 0 {
            return Ok(0);
        }

        buf.truncate(in_len as usize);
        let out_offset = if !off_out.is_null() {
            let off_out = off_out.read().await?;
            if off_out < 0 {
                return Err(Errno::EINVAL);
            }
            off_out as usize
        } else {
            0
        };
        let out_len = file_out.write_at(out_offset, &buf).await? as usize;

        if !off_in.is_null() {
            off_in.write(off_in.read().await? + in_len as i64).await?;
        }
        if !off_out.is_null() {
            off_out
                .write(off_out.read().await? + out_len as i64)
                .await?;
        }

        Ok(out_len as isize)
    }
}

/// create if not exist
/// and the created file/dir is NON-NEGATIVE
///
/// todo: add function: create with the last file negative
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
                .path()?;
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            Ok(cwd.from_cd_or_create(&path_str, mode).await)
        }
    } else {
        Ok(Path::from_or_create(path_str, mode).await)
    }
}

fn get_path(
    task: Arc<Task>,
    rawpath: usize,
    fd: isize,
    debug_syscall_name: &str,
) -> SysResult<Path> {
    let path_str = get_string_from_ptr(&UserPtr::<u8>::new(rawpath));
    debug!("[{debug_syscall_name}] path: {}", path_str);
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
                .path()?;
            trace!("[{debug_syscall_name}] cwd: {:?}", cwd);
            cwd.from_cd(&path_str)
        }
    } else {
        Path::try_from(path_str)
    }
}

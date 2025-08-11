use alloc::{string::String, sync::Arc};
use core::intrinsics::unlikely;

use include::errno::SysResult;
use ksync::assert_no_lock;

use super::{Syscall, SyscallResult};
use crate::{
    constant::fs::{
        AT_EACCESS, AT_FDCWD, F_OK, R_OK, STATX_ALL, STATX_ATIME, STATX_BLOCKS, STATX_BTIME,
        STATX_CTIME, STATX_GID, STATX_INO, STATX_MODE, STATX_MTIME, STATX_NLINK, STATX_SIZE,
        STATX_TYPE, STATX_UID, UTIME_NOW, UTIME_OMIT, W_OK, X_OK,
    },
    fs::{
        fdtable::RLimit,
        path::{get_dentry, get_dentry_parent, kcreate_async},
        pipe::PipeFile,
    },
    include::{
        fs::{
            AtFlags, BlkIoctlCmd, DevT, FallocFlags, FcntlFlags, FdFlags, FileFlags, Flock,
            InodeMode, IoctlCmd, Iovec, Kstat, LoopIoctlCmd, MountFlags, NoAxiomIoctlCmd,
            RenameFlags, RtcIoctlCmd, SearchFlags, SeekFrom, Statfs, Statx, TtyIoctlCmd, Whence,
            EXT4_MAX_FILE_SIZE,
        },
        resource::Resource,
        result::Errno,
        time::TimeSpec,
    },
    mm::user_ptr::UserPtr,
    return_errno,
    signal::interruptable::interruptable,
    time::gettime::get_time_duration,
    utils::{
        global_alloc,
        hack::{switch_into_ltp, switch_outof_ltp},
        log::{switch_log_off, switch_log_on},
    },
};

impl Syscall<'_> {
    /// Get current working directory
    pub async fn sys_getcwd(&self, buf: usize, size: usize) -> SyscallResult {
        let cwd = self.task.cwd().clone();
        let cwd_str = format!("{}\0", cwd.path());
        let cwd_bytes = cwd_str.as_bytes();
        if cwd_bytes.len() > size {
            return Err(Errno::ERANGE);
        }

        info!(
            "[sys_getcwd] buf: {:?}, size: {}, cwd:{:?}",
            buf, size, cwd_str
        );

        let user_ptr = UserPtr::<u8>::new(buf);
        if user_ptr.is_null() {
            error!("[sys_getcwd] invalid user pointer");
            return Err(Errno::EFAULT);
        }
        let buf_slice = user_ptr.as_slice_mut_checked(size).await?;
        buf_slice[..cwd_bytes.len()].copy_from_slice(cwd_bytes);

        Ok(buf as isize)
    }

    /// Create a pipe
    pub async fn sys_pipe2(&self, pipe: usize, flags: i32) -> SyscallResult {
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        let (read_end, write_end) = PipeFile::new_pipe(&flags);
        let flags = FdFlags::from(&flags);

        let user_ptr = UserPtr::<i32>::new(pipe);
        let buf_slice = user_ptr.as_slice_mut_checked(2).await?;

        let mut fd_table = self.task.fd_table();
        let read_fd = fd_table.alloc_fd()?;
        fd_table.set(read_fd, read_end);
        fd_table.set_fdflag(read_fd, &flags);
        buf_slice[0] = read_fd as i32;

        let write_fd = fd_table.alloc_fd()?;
        fd_table.set(write_fd, write_end);
        fd_table.set_fdflag(write_fd, &flags);
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
    pub fn sys_dup3(&self, old_fd: usize, new_fd: usize, flags: i32) -> SyscallResult {
        info!("[sys_dup3] old_fd: {}, new_fd: {}", old_fd, new_fd);

        if (flags & !(FileFlags::O_CLOEXEC.bits())) != 0 {
            return Err(Errno::EINVAL);
        }
        if new_fd == old_fd {
            info!("[sys_dup3] same fd!");
            return Err(Errno::EINVAL);
        }

        let mut fd_table = self.task.fd_table();
        if fd_table.get(old_fd).is_none() {
            return Err(Errno::EBADF);
        }

        fd_table.fill_to(new_fd)?;
        fd_table.copyfrom(old_fd, new_fd)?;

        if flags & FileFlags::O_CLOEXEC.bits() != 0 {
            fd_table.set_fdflag(new_fd, &FdFlags::FD_CLOEXEC);
        }

        Ok(new_fd as isize)
    }

    /// Switch to a new work directory
    pub fn sys_chdir(&self, path: usize) -> SyscallResult {
        let path = read_path(path)?;
        info!("[sys_chdir] path: {}", path);

        let searchflags = SearchFlags::empty();
        let dentry = get_dentry(self.task, AT_FDCWD, &path, &searchflags)?;
        let inode = dentry.inode()?;
        if inode.file_type() != InodeMode::DIR {
            error!("[sys_chdir] tar path must be a dir");
            return Err(Errno::ENOTDIR);
        }
        dentry.check_arrive()?;
        *self.task.cwd() = dentry;
        Ok(0)
    }

    pub fn sys_fchdir(&self, fd: usize) -> SyscallResult {
        info!("[sys_fchdir] fd: {}", fd);
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        let dentry = file.dentry();
        dentry.check_access(self.task, X_OK, true)?;
        *self.task.cwd() = dentry;
        Ok(0)
    }

    /// Change the root directory of the process
    pub fn sys_chroot(&self, path: usize) -> SyscallResult {
        let path = read_path(path)?;
        info!("[sys_chroot] path: {:?}", path);
        let searchflags = SearchFlags::empty();
        let dentry = get_dentry(self.task, AT_FDCWD, &path, &searchflags)?;
        let inode = dentry.inode()?;
        if inode.file_type() != InodeMode::DIR {
            error!("[sys_chroot] chroot path must be a dir");
            return Err(Errno::ENOTDIR);
        }
        if !inode.check_search_permission(self.task) {
            error!("[sys_chroot] chroot path must be searchable");
            return Err(Errno::EACCES);
        }
        if self.task.fsuid() != 0 {
            error!("[sys_chroot] only root can chroot");
            return Err(Errno::EPERM);
        }
        *self.task.root() = dentry;
        Ok(0)
    }

    /// Open or create a file
    pub async fn sys_openat(&self, fd: isize, path: usize, flags: i32, mode: u32) -> SyscallResult {
        let path_str = UserPtr::<u8>::new(path).get_string_from_ptr()?;
        let mode = InodeMode::from_bits(mode).ok_or(Errno::EINVAL)?;
        let searchflags = SearchFlags::from_bits_truncate(flags);
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_openat] dirfd {}, flags {:?}, filename {}, mode {:?}",
            fd, flags, path_str, mode
        );

        let path = read_path(path)?;
        let fd_flags = FdFlags::from(&flags);

        let dentry = if flags.contains(FileFlags::O_TMPFILE) {
            let this = get_dentry(self.task, fd, &path, &searchflags)?;
            if this.inode()?.file_type() != InodeMode::DIR {
                error!("[sys_openat] O_TMPFILE can only be used on directories");
                return Err(Errno::ENOTDIR);
            }
            let tmpname = format!("/tmp/kernel_O_TMPFILE_{}", global_alloc());
            let dentry = kcreate_async(&tmpname, mode | InodeMode::FILE).await;
            Ok(dentry)
        } else if flags.contains(FileFlags::O_CREATE) {
            let (dentry, name) = get_dentry_parent(self.task, fd, &path, &searchflags)?;
            if !dentry.can_search(self.task) {
                return Err(Errno::EACCES);
            }
            if let Some(dentry) = dentry.get_child(name) {
                Ok(dentry)
            } else {
                dentry.clone().create(name, mode | InodeMode::FILE).await
            }
        } else {
            get_dentry(self.task, fd, &path, &searchflags)
        }?;

        let inode = dentry.inode()?;
        if flags.contains(FileFlags::O_TRUNC) {
            inode.set_size(0);
            inode.truncate(0).await?;
        }
        if flags.contains(FileFlags::O_DIRECTORY) && inode.file_type() != InodeMode::DIR {
            return Err(Errno::ENOTDIR);
        }

        let file = dentry.open(&flags)?;
        let file_path = file.dentry().path();
        let mut fd_table = self.task.fd_table();
        let fd = fd_table.alloc_fd()?;
        fd_table.set(fd, file);
        fd_table.set_fdflag(fd, &fd_flags);

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

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(len).await?;

        if !file.meta().readable() {
            return Err(Errno::EINVAL);
        }

        let read_fut = file.read(buf_slice);
        if file.is_interruptable() {
            interruptable(self.task, read_fut, None, None).await?
        } else {
            read_fut.await
        }
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
            iov_ptr.as_slice_const_checked(Iovec::size()).await?;

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
        let buf_slice = user_ptr.as_slice_const_checked(len).await?;

        if !file.meta().writable() {
            return Err(Errno::EINVAL);
        }

        let write_fut = file.write(buf_slice);
        if file.is_interruptable() {
            interruptable(self.task, write_fut, None, None).await?
        } else {
            write_fut.await
        }
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
            iov_ptr.as_slice_const_checked(Iovec::size()).await?;

            let iov = iov_ptr.read().await?;
            if iov.iov_len == 0 {
                continue;
            }
            let buf_ptr = UserPtr::<u8>::new(iov.iov_base);
            let buf_slice = buf_ptr.as_slice_const_checked(iov.iov_len).await?;
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
        let buf_slice = user_ptr.as_slice_const_checked(len).await?;
        file.write_at(offset, buf_slice).await
    }

    /// Create a directory
    pub async fn sys_mkdirat(&self, dirfd: isize, path: usize, mode: u32) -> SyscallResult {
        let mode = InodeMode::from_bits_truncate(mode); // todo: is the mode correct?
        let path = read_path(path)?;
        info!(
            "[sys_mkdirat] dirfd: {}, path: {:?}, mode: {:?}",
            dirfd, path, mode
        );

        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        let (dentry, name) = get_dentry_parent(self.task, dirfd, &path, &searchflags)?;
        if let Some(_) = dentry.get_child(name) {
            warn!("[sys_mkdirat] dir already exists: {}", name);
            return Err(Errno::EEXIST);
        } else {
            dentry.clone().create(name, mode | InodeMode::DIR).await?;
        }
        Ok(0)
    }

    /// Close a file
    pub fn sys_close(&self, fd: usize) -> SyscallResult {
        info!("[sys_close] fd: {}", fd);
        let mut fd_table = self.task.fd_table();
        if let Some(file) = fd_table.get(fd) {
            debug!("[sys_close] closing file: {:?}", file.name());
        }
        fd_table.close(fd)
    }

    /// Get file status
    pub async fn sys_fstat(&self, fd: usize, stat_buf: usize) -> SyscallResult {
        trace!("[sys_fstat]: fd: {}, stat_buf: {:#x}", fd, stat_buf);

        // Check for NULL pointer
        if stat_buf == 0 {
            return Err(Errno::EFAULT);
        }

        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_fstat] file_name: {:?}", file_name);
        let kstat = Kstat::from_stat(file.inode())?;
        let ptr = UserPtr::<Kstat>::new(stat_buf);
        ptr.write(kstat).await?;
        Ok(0)
    }

    pub async fn sys_statx(
        &self,
        dirfd: isize,
        path: usize,
        flags: i32,
        mask: u32,
        buf: usize,
    ) -> SyscallResult {
        let path = read_path(path)?;
        if flags < 0 {
            error!("[sys_statx] invalid flags: {}", flags);
            return Err(Errno::EINVAL);
        }
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_statx] dirfd: {}, path: {:?}, flags: {:?}",
            dirfd, path, flags,
        );
        let dentry = if flags.contains(AtFlags::AT_EMPTY_PATH) {
            self.task
                .fd_table()
                .get(dirfd as usize)
                .ok_or(Errno::EBADF)?
                .dentry()
        } else {
            if path.is_empty() {
                error!("[sys_statx] pathname is empty");
                return Err(Errno::ENOENT);
            }
            get_dentry(self.task, dirfd, &path, &flags.into())?
        };

        const VALID_MASK: u32 = STATX_TYPE
            | STATX_MODE
            | STATX_NLINK
            | STATX_UID
            | STATX_GID
            | STATX_ATIME
            | STATX_MTIME
            | STATX_CTIME
            | STATX_INO
            | STATX_SIZE
            | STATX_BLOCKS
            | STATX_BTIME
            | STATX_ALL;
        if (mask & !VALID_MASK) != 0 {
            error!("[sys_statx] invalid mask: {}", mask);
            return Err(Errno::EINVAL);
        }

        let statx = dentry.inode()?.statx(mask)?;
        let ptr = UserPtr::<Statx>::new(buf);
        if ptr.is_null() {
            error!("[statx] invalid user pointer");
            return Err(Errno::EFAULT);
        }
        ptr.write(statx).await?;
        Ok(0)
    }

    /// Get file status
    pub async fn sys_fstatat(
        &self,
        dirfd: isize,
        path: usize,
        stat_buf: usize,
        flags: i32,
    ) -> SyscallResult {
        let path = read_path(path)?;
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_newfstat] dirfd: {}, path: {:?}, stat_buf: {:#x}, flags: {:?}",
            dirfd, path, stat_buf, flags
        );
        let dentry = get_dentry(self.task, dirfd, &path, &flags.into())?;
        let kstat = Kstat::from_stat(dentry.inode()?)?;
        let ptr = UserPtr::<Kstat>::new(stat_buf);
        ptr.write(kstat).await?;
        Ok(0)
    }

    /// get fs status
    pub async fn sys_statfs(&self, path: usize, buf: usize) -> SyscallResult {
        let path = read_path(path)?;
        let searchflags = SearchFlags::empty();
        let dentry = get_dentry(self.task, AT_FDCWD, &path, &searchflags)?;
        info!("[sys_statfs] path: {}", path);
        let statfs = dentry.super_block().statfs().await?;
        let ptr = UserPtr::<Statfs>::new(buf);
        ptr.write(statfs).await?;
        Ok(0)
    }

    pub async fn sys_fstatfs(&self, fd: usize, buf: usize) -> SyscallResult {
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        let dentry = file.dentry();
        info!("[sys_fstatfs] fd: {}, path: {}", fd, dentry.path());
        let statfs = dentry.super_block().statfs().await?;
        let ptr = UserPtr::<Statfs>::new(buf);
        ptr.write(statfs).await?;
        Ok(0)
    }

    pub async fn sys_ioctl(&self, fd: usize, request: usize, arg: usize) -> SyscallResult {
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        // let file_name = file.dentry().path()?;
        // info!("[sys_ioctl] file_name: {:?}", file_name);

        let cmd = if let Some(cmd) = TtyIoctlCmd::from_repr(request) {
            IoctlCmd::Tty(cmd)
        } else if let Some(cmd) = RtcIoctlCmd::from_repr(request) {
            IoctlCmd::Rtc(cmd)
        } else if let Some(cmd) = LoopIoctlCmd::from_repr(request) {
            IoctlCmd::Loop(cmd)
        } else if let Some(cmd) = BlkIoctlCmd::from_repr(request) {
            IoctlCmd::Block(cmd)
        } else if let Some(cmd) = NoAxiomIoctlCmd::from_repr(request) {
            IoctlCmd::Other(cmd)
        } else {
            error!("Unknown ioctl command: {:#x}", request);
            return Err(Errno::EINVAL);
        };
        info!(
            "[sys_ioctl]: fd: {}, request: {:#x}, argp: {:#x}, cmd: {:?}",
            fd, request, arg, cmd
        );
        match cmd {
            IoctlCmd::Tty(_) => {
                return file.ioctl(request, arg);
            }
            IoctlCmd::Rtc(x) => match x {
                RtcIoctlCmd::RTCRDTIME => {
                    return file.ioctl(request, arg);
                }
            },
            IoctlCmd::Loop(_) => {
                return file.ioctl(request, arg);
            }
            IoctlCmd::Block(x) => match x {
                BlkIoctlCmd::RVBLKGETSIZE64 | BlkIoctlCmd::LABLKGETSIZE64 => {
                    let ptr = UserPtr::<u64>::new(arg);
                    ptr.write(0x10000).await?;
                    return Ok(0);
                }
            },
            IoctlCmd::Other(x) => match x {
                NoAxiomIoctlCmd::TESTCASE => {
                    const IOCTL_SWITCH_INTO_LTP: usize = 0;
                    const IOCTL_SWITCH_OUTOF_LTP: usize = 1;
                    match arg {
                        IOCTL_SWITCH_INTO_LTP => {
                            switch_into_ltp();
                            println_debug!("[kernel] into testcase")
                        }
                        IOCTL_SWITCH_OUTOF_LTP => {
                            switch_outof_ltp();
                            println_debug!("[kernel] out of testcase");
                        }
                        _ => {
                            return_errno!(Errno::EINVAL, "arg {} is not supported", arg);
                        }
                    }
                }
                NoAxiomIoctlCmd::LOG => {
                    const IOCTL_LOG_OFF: usize = 0;
                    const IOCTL_LOG_ON: usize = 1;
                    match arg {
                        IOCTL_LOG_OFF => {
                            println_debug!("[kernel] log off");
                            switch_log_off();
                        }
                        IOCTL_LOG_ON => {
                            switch_log_on();
                            println_debug!("[kernel] log on");
                        }
                        _ => {
                            return_errno!(Errno::EINVAL, "arg {} is not supported", arg);
                        }
                    }
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
        assert_no_lock!();
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
        let special = ptr.get_string_from_ptr()?;
        let ptr = UserPtr::<u8>::new(dir);
        let dir = ptr.get_string_from_ptr()?;
        let ptr = UserPtr::<u8>::new(fstype);
        let fstype = ptr.get_string_from_ptr()?;
        let flags = MountFlags::from_bits(flags as u32).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_mount] [NO IMPL] special: {}, dir: {}, fstype: {}, flags: {:?}",
            special, dir, fstype, flags
        );
        Ok(0)
    }

    /// Unmount a file system
    pub fn sys_umount2(&self, dir: usize, flags: usize) -> SyscallResult {
        let ptr = UserPtr::<u8>::new(dir);
        let dir = ptr.get_string_from_ptr()?;
        info!("[sys_umount2] [NO IMPL] target: {}, flags: {}", dir, flags);
        Ok(0)
    }

    /// Create a hard link
    pub async fn sys_linkat(
        &self,
        olddirfd: isize,
        oldpath: usize,
        newdirfd: isize,
        newpath: usize,
        flags: i32,
    ) -> SyscallResult {
        let flags = AtFlags::from_bits_truncate(flags);
        let old_path = read_path(oldpath)?;
        let new_path = read_path(newpath)?;
        info!(
            "[sys_linkat] olddirfd: {}, oldpath: {}, newdirfd: {}, newpath: {}, flags: {:?}",
            olddirfd, old_path, newdirfd, new_path, flags
        );

        if old_path == "/proc/meminfo" || old_path == "/proc/cpuinfo" {
            // todo: support /proc
            return Err(Errno::EXDEV);
        }

        if flags.bits() & !(AtFlags::AT_SYMLINK_FOLLOW.bits() | AtFlags::AT_EMPTY_PATH.bits()) != 0
        {
            error!("[sys_linkat] invalid flags: {:?}", flags);
            return Err(Errno::EINVAL);
        }

        let searchflags = if flags.contains(AtFlags::AT_SYMLINK_FOLLOW) {
            SearchFlags::empty()
        } else {
            SearchFlags::AT_SYMLINK_NOFOLLOW
        };

        let old_dentry = get_dentry(self.task, olddirfd, &old_path, &searchflags)?;
        if old_dentry.inode()?.file_type() == InodeMode::DIR {
            error!("[sys_linkat] old_dentry is directory");
            return Err(Errno::EPERM);
        }

        let target_dentry = old_dentry;
        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        let (parent, name) = get_dentry_parent(self.task, newdirfd, &new_path, &searchflags)?;
        parent.check_access(self.task, W_OK, true)?;
        parent.create_link(target_dentry, name).await?;
        Ok(0)
    }

    pub async fn sys_symlinkat(
        &self,
        target: usize,
        newdirfd: isize,
        linkpath: usize,
    ) -> SyscallResult {
        let target_path = read_path(target)?;
        let link_path = read_path(linkpath)?;
        info!(
            "[sys_symlinkat] target: {}, newdirfd: {}, linkpath: {}",
            target_path, newdirfd, link_path
        );

        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        let (parent, name) = get_dentry_parent(self.task, newdirfd, &link_path, &searchflags)?;
        parent.create_symlink(target_path, name).await?;
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
        if buflen as isize <= 0 {
            error!("[sys_readlinkat] buflen must be greater than 0");
            return Err(Errno::EINVAL);
        }
        let path = read_path(path)?;

        info!(
            "[sys_readlinkat] dirfd: {}, path: {:?}, buf: {:#x}, bufsize: {}",
            dirfd, path, buf, buflen,
        );

        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        let dentry = if path.is_empty() {
            self.task
                .fd_table()
                .get(dirfd as usize)
                .ok_or(Errno::EBADF)?
                .dentry()
        } else {
            get_dentry(self.task, dirfd, &path, &searchflags)?
        };

        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;

        if dentry.name() == "exe" {
            let file = dentry.open(&FileFlags::empty())?;
            return file.base_readlink(buf_slice).await;
        }

        if let Ok(inode) = dentry.inode() {
            if let Some(tar_path) = inode.symlink() {
                let len = tar_path.len();
                if buflen < len {
                    error!(
                        "[sys_readlinkat] buffer size {} is too small for symlink {}",
                        buflen, tar_path
                    );
                    return Err(Errno::ENAMETOOLONG);
                }
                debug!(
                    "[sys_readlinkat] symlink {} resolved to {}",
                    dentry.name(),
                    tar_path
                );
                buf_slice[..len].copy_from_slice(tar_path.as_bytes());
                Ok(len as isize)
            } else {
                error!(
                    "[sys_readlinkat] dentry is not a symlink: {}",
                    dentry.name()
                );
                Err(Errno::EINVAL)
            }
        } else {
            error!("[sys_readlinkat] dentry {} is negative.", dentry.name());
            Err(Errno::ENOENT)
        }
    }

    /// Unlink a file, also delete the file if nlink is 0
    pub async fn sys_unlinkat(&self, dirfd: isize, path: usize, flags: i32) -> SyscallResult {
        pub const AT_REMOVEDIR: i32 = 0x200;
        if flags & !AT_REMOVEDIR != 0 {
            error!("[sys_unlinkat] AT_REMOVEDIR flag is set");
            return Err(Errno::EINVAL);
        }
        let path = read_path(path)?;
        let flags = FileFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        info!(
            "[sys_unlinkat] dirfd: {}, path: {}, flags: {:?}",
            dirfd, path, flags
        );
        let dentry = get_dentry(self.task, dirfd, &path, &searchflags)?;
        if unlikely(dentry.name() == "interrupts") {
            // MENTION: this is required by official
            return Err(Errno::ENOSYS);
        }
        let inode = if let Ok(inode) = dentry.inode() {
            inode
        } else {
            warn!("[sys_unlinkat] {} is negative", dentry.name());
            return Ok(0);
        };
        let mut nlink = inode.meta().inner.lock().nlink;
        debug!(
            "[sys_unlinkat] nlink: {}, file_type: {:?}",
            nlink,
            inode.file_type()
        );
        nlink -= 1;
        if nlink == 0 {
            let parent = dentry.parent().unwrap();
            parent.check_access(self.task, W_OK, true)?;
            let mut w_guard = crate::fs::pagecache::get_pagecache_wguard();
            let file = dentry.clone().open(&flags)?;
            w_guard.mark_deleted(&file);
            drop(w_guard);
            inode
                .set_state(crate::fs::vfs::basic::inode::InodeState::Deleted)
                .await;
            parent.remove_child(&dentry.name()).unwrap();
            parent
                .open(&FileFlags::empty())
                .unwrap()
                .delete_child(&dentry.name())
                .await?;
        }
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
                    _ => RLimit::default(), //TODO: add rlimit for Task
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
            match resource {
                Resource::NOFILE => *fd_table.rlimit_mut() = new_limit.read().await?,
                _ => {}
            }
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
    pub async fn sys_fcntl(&self, fd: usize, cmd: usize, arg: usize) -> SyscallResult {
        let task = self.task;
        let flags = FileFlags::from_bits_retain(arg as i32);
        let mut fd_table = task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        // let file_name = file.dentry().path()?;
        // info!("[sys_fcntl] file_name: {:?}", file_name);
        let op = FcntlFlags::from_repr(cmd).ok_or(Errno::EINVAL)?;

        info!("[sys_fcntl] fd: {fd}, cmd: {op:?}, arg: {flags:?}");
        match op {
            FcntlFlags::F_SETFL => {
                file.meta().set_flags(flags);
                Ok(0)
            }
            FcntlFlags::F_SETFD => {
                let arg = FileFlags::from_bits_retain(arg as i32);
                let fd_flags = FdFlags::from(&arg);
                fd_table.set_fdflag(fd, &fd_flags);
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
                let new_fd = fd_table.alloc_fd_after(arg)?;
                assert!(new_fd > fd);
                fd_table.copyfrom(fd, new_fd)
            }
            FcntlFlags::F_DUPFD_CLOEXEC => {
                let new_fd = fd_table.alloc_fd_after(arg)?;
                assert!(new_fd > fd);
                let ret = fd_table.copyfrom(fd, new_fd)?;
                fd_table.set_fdflag(new_fd, &FdFlags::FD_CLOEXEC);
                Ok(ret)
            }
            FcntlFlags::F_SETLK => {
                let ptr = UserPtr::<Flock>::new(arg);
                if ptr.is_null() {
                    return Err(Errno::EFAULT);
                } else {
                    let flock = ptr.read().await?;
                    debug!("[flock] {:?}", flock);
                }
                return Err(Errno::EINVAL);
            }
            _ => {
                warn!("fcntl cmd: {op:?} not implemented");
                Ok(0)
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
            let read_fut = in_file.read_at(offset, &mut buf);
            let read_len = if in_file.is_interruptable() {
                interruptable(self.task, read_fut, None, None).await?
            } else {
                read_fut.await
            }? as usize;
            offset_ptr.write(offset + read_len).await?;
            read_len
        } else {
            let read_fut = in_file.read(&mut buf);
            if in_file.is_interruptable() {
                interruptable(self.task, read_fut, None, None).await?
            } else {
                read_fut.await
            }? as usize
        };

        let write_fut = out_file.write(&buf[..read_len]);
        if out_file.is_interruptable() {
            interruptable(self.task, write_fut, None, None).await?
        } else {
            write_fut.await
        }
    }

    /// Modify timestamp of a file
    pub async fn sys_utimensat(
        &self,
        dirfd: isize,
        path: usize,
        times: usize,
        flags: i32,
    ) -> SyscallResult {
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_utimensat] dirfd: {}, times: {:#x}, flags: {:?}",
            dirfd, times, flags
        );
        let path_ptr = UserPtr::<u8>::new(path);
        let times_ptr = UserPtr::<TimeSpec>::new(times);
        let inode = if path_ptr.is_null() {
            match dirfd {
                AT_FDCWD => {
                    error!("[sys_utimensat] dirfd is AT_FDCWD but path is null");
                    return Err(Errno::EINVAL);
                }
                fd => self
                    .task
                    .fd_table()
                    .get(fd as usize)
                    .ok_or(Errno::EBADF)?
                    .inode(),
            }
        } else {
            let path = read_path(path)?;
            let dentry = get_dentry(self.task, dirfd, &path, &flags.into())?;
            dentry.inode()?
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
            Whence::SeekData => {
                error!("[sys_lseek] SeekData is not implemented, offset not changed");
                return Ok(file.pos() as isize);
            }
            Whence::SeekHold => {
                error!("[sys_lseek] SeekHold is not implemented, offset not changed");
                return Ok(file.pos() as isize);
            }
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
        let old_path = read_path(old_path)?;
        let new_path = read_path(new_path)?;
        let flags = RenameFlags::from_bits(flags).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_renameat2] old_path: {:?}, new_path: {:?}",
            old_path, new_path
        );

        if (flags.contains(RenameFlags::RENAME_NOREPLACE)
            || flags.contains(RenameFlags::RENAME_WHITEOUT))
            && flags.contains(RenameFlags::RENAME_EXCHANGE)
        {
            error!("[sys_renameat2] NOREPLACE and RENAME_EXCHANGE cannot be used together");
            return Err(Errno::EINVAL);
        }

        let searchflags = SearchFlags::empty();
        let old_dentry = get_dentry(self.task, old_dirfd, &old_path, &searchflags)?;
        let (new_dentry_parent, new_name) =
            get_dentry_parent(self.task, new_dirfd, &new_path, &searchflags)?;

        if old_dentry.is_negative() {
            error!("[sys_renameat2] oldpath does not exist");
            return Err(Errno::ENOENT);
        }

        if let Some(new_dentry) = new_dentry_parent.get_child(new_name) {
            // new dentry must not exist if NOREPLACE is set
            if flags.contains(RenameFlags::RENAME_NOREPLACE) {
                error!("[sys_renameat2] newpath already exists with NOREPLACE flag");
                return Err(Errno::EINVAL);
            }

            // check if newpath is an ancestor of oldpath
            if flags.contains(RenameFlags::RENAME_EXCHANGE) {
                let mut current = old_dentry.clone();
                loop {
                    if let Some(parent) = current.parent() {
                        if core::ptr::addr_eq(Arc::as_ptr(&parent), Arc::as_ptr(&new_dentry)) {
                            error!("[sys_renameat2] newpath is an ancestor of oldpath");
                            return Err(Errno::EINVAL);
                        }
                        current = parent;
                    } else {
                        break;
                    }
                }
            }

            let new_inode = new_dentry.inode().expect("should have inode");

            // fixme: if old_dentry is a dir, new_dentry must not exist or be an empty dir
            // currently doesn't check if it is an empty dir
            if old_dentry.inode().unwrap().file_type() == InodeMode::DIR {
                error!("[sys_renameat2] new_dentry is not an empty directory");
                return Err(Errno::ENOTEMPTY);
            }

            // check if old_dentry and new_dentry are the same hard link
            if Arc::ptr_eq(&old_dentry.inode().unwrap(), &new_inode) {
                warn!(
                    "[sys_renameat2] old_dentry and new_dentry are the hard links of the same file"
                );
                return Ok(0);
            }
        } else {
            // new_path does not exist, but if EXCHANGE is set
            if flags.contains(RenameFlags::RENAME_EXCHANGE) {
                error!("[sys_renameat2] newpath must exist with EXCHANGE flag");
                return Err(Errno::EINVAL);
            }
        }

        // rename: currently just create a new file with the same inode and delete the
        // old one
        // todo: support the real fs rename, mention that ext4_rs doesn't support rename
        let parent = old_dentry.parent().ok_or(Errno::ENOENT)?;
        let old_inode = old_dentry.inode().unwrap();
        parent
            .clone()
            .create(new_name, old_inode.inode_mode())
            .await?;
        let new_dentry = parent.get_child(new_name).unwrap();
        new_dentry.set_inode(old_inode);

        let old_name = old_dentry.name();
        parent
            .clone()
            .open(&FileFlags::empty())
            .unwrap()
            .delete_child(old_name)
            .await?;
        parent.remove_child(old_dentry.name()).unwrap();

        Ok(0)
    }

    /// Truncate a file to a specified length
    /// todo: FIXME!
    pub async fn sys_ftruncate(&self, fd: usize, length: usize) -> SyscallResult {
        info!("[sys_ftruncate] fd: {}, length: {}", fd, length);
        let fd_table = self.task.fd_table();
        let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
        drop(fd_table);
        let mode = file.inode().inode_mode();
        if mode.contains(InodeMode::SOCKET) {
            error!("[sys_ftruncate] ftruncate on a socket");
            return Err(Errno::EINVAL);
        }
        if mode.contains(InodeMode::DIR) {
            error!("[sys_ftruncate] ftruncate on a directory");
            return Err(Errno::EISDIR);
        }
        if !file.meta().writable() {
            error!("[sys_ftruncate] file is not writable");
            return Err(Errno::EINVAL);
        }
        // let file_name = file.dentry().path()?;
        // info!("[sys_ftruncate] file_name: {:?}", file_name);
        file.dentry().check_access(self.task, W_OK, true)?;
        let size = file.size();
        if size < length {
            if length - size <= 4096 {
                // fixme: where are not clean?
                file.write_at(size, &vec![0u8; length - size]).await?;
            }
            file.inode().set_size(length);
            Ok(0)
        } else {
            file.truncate_pagecache(length);
            file.inode().set_size(length);
            file.inode().truncate(length).await?;
            Ok(0)
        }
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

    pub fn sys_fchmod(&self, fd: usize, mode: usize) -> SyscallResult {
        info!("[sys_fchmod] set {:o} mode for fd: {}", mode, fd);
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        if file.flags().contains(FileFlags::O_PATH) {
            error!("[sys_fchmod] O_PATH file cannot be changed");
            return Err(Errno::EINVAL);
        }
        let inode = file.dentry().inode()?;
        inode.set_permission(self.task, mode as u32);
        Ok(0)
    }

    pub fn sys_fchmodat(&self, fd: usize, path: usize, mode: usize, flags: i32) -> SyscallResult {
        let path = read_path(path)?;
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_fchmodat] set {:o} mode to {:?}, flags: {:?}",
            mode, path, flags
        );
        if path.is_empty() {
            error!("[sys_fchmodat] path is empty");
            return Err(Errno::ENOENT);
        }

        let dentry = get_dentry(self.task, fd as isize, &path, &flags.into())?;
        let inode = dentry.inode()?;
        inode.set_permission(self.task, mode as u32);
        Ok(0)
    }

    pub fn sys_fchown(&self, fd: usize, owner: u32, group: u32) -> SyscallResult {
        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        info!(
            "[sys_fchown] set owner: {:?}, group: {:?} for file: {}",
            owner,
            group,
            file.name()
        );
        file.inode().chown(self.task, owner, group)?;
        Ok(0)
    }

    pub fn sys_fchownat(
        &self,
        fd: usize,
        path: usize,
        owner: u32,
        group: u32,
        flags: i32,
    ) -> SyscallResult {
        let path = read_path(path)?;
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_fchownat] set owner: {:?}, group: {:?} for {:?}, flags: {:?}",
            owner, group, path, flags
        );

        let dentry = get_dentry(self.task, fd as isize, &path, &flags.into())?;
        let inode = dentry.inode()?;
        inode.chown(self.task, owner, group)?;
        Ok(0)
    }

    /// check user's permissions of a file relative to a directory file
    /// descriptor
    pub fn sys_faccessat(&self, fd: usize, path: usize, mode: i32, flags: i32) -> SyscallResult {
        let is_fs = flags & AT_EACCESS != 0;
        let path = read_path(path)?;
        let supported_flags = AT_EACCESS | 0x1000 | 0x100 | 0x60 | 0x2 | 0xfffe | 0x1;
        if flags & !supported_flags != 0 {
            log::error!("[sys_faccessat] Unsupported flags: {:#x}", flags);
            return Err(Errno::EINVAL);
        }
        let flags = AtFlags::from_bits_truncate(flags);
        info!(
            "[sys_faccessat] fd: {}, path: {:?}, mode: {}, flags: {:?}",
            fd, path, mode, flags
        );

        let dentry = get_dentry(self.task, fd as isize, &path, &flags.into())?;

        if mode & !(F_OK | R_OK | W_OK | X_OK) != 0 {
            error!("[sys_faccessat] shouldn't have mode: {:?}", mode);
            return Err(Errno::EINVAL);
        }
        dentry.check_access(self.task, mode, is_fs)?;
        Ok(0)
    }

    pub async fn sys_copy_file_range(
        &self,
        fd_in: usize,
        off_in: usize,
        fd_out: usize,
        off_out: usize,
        len: usize,
        flags: i32,
    ) -> SyscallResult {
        if flags != 0 {
            return Err(Errno::EINVAL);
        }
        let fd_table = self.task.fd_table();
        let in_file = fd_table.get(fd_in).ok_or(Errno::EBADF)?;
        let out_file = fd_table.get(fd_out).ok_or(Errno::EBADF)?;
        drop(fd_table);

        if !in_file.meta().readable() || !out_file.meta().writable() {
            return Err(Errno::EBADF);
        }
        let off_in = UserPtr::<u32>::new(off_in);
        let off_in = off_in.get_ref_mut().await?;
        let off_out = UserPtr::<u32>::new(off_out);
        let off_out = off_out.get_ref_mut().await?;

        info!(
            "[sys_copy_file_range] fd_in: {}, off_in: {:?}, offset: {}, size: {}   fd_out: {}, off_out: {:?}, offset: {}, size: {}, len: {}",
            fd_in,
            off_in,
            in_file.pos(),
            in_file.size(),
            fd_out,
            off_out,
            out_file.pos(),
            out_file.size(),
            len
        );

        // the write will always be successful
        let in_file_size = in_file.size();
        let in_file_offset = in_file.pos();
        let ret_len = if off_in.is_none() {
            if in_file_size <= in_file_offset {
                return Ok(0);
            }
            in_file_size - in_file_offset
        } else {
            let off_in_value = **(off_in.as_ref().unwrap()) as usize;
            if in_file_size <= off_in_value {
                return Ok(0);
            }
            in_file_size - off_in_value
        };
        let ret_len = ret_len.min(len);

        let mut buf = vec![0u8; ret_len];
        if off_in.is_none() {
            in_file.read(&mut buf).await?;
        } else {
            in_file
                .read_at(**(off_in.as_ref().unwrap()) as usize, &mut buf)
                .await?;
            *off_in.unwrap() += ret_len as u32;
        }

        if off_out.is_none() {
            out_file.write(&buf).await?;
        } else {
            out_file
                .write_at(**(off_out.as_ref().unwrap()) as usize, &buf)
                .await?;
            *off_out.unwrap() += ret_len as u32;
        }

        let current_time = TimeSpec::from(get_time_duration());
        out_file.inode().set_time(
            &Some(current_time),
            &Some(current_time),
            &Some(current_time),
        );

        debug!(
            "[sys_copy_file_range] fd_in: {}, off_in: xx, offset: {}, size: {}   fd_out: {}, off_out: xx, offset: {}, size: {}, len: {}",
            fd_in,
            in_file.pos(),
            in_file.size(),
            fd_out,
            out_file.pos(),
            out_file.size(),
            len
        );

        Ok(ret_len as isize)
    }

    /// Allocate space for a file, similar to fallocate
    /// todo: implement the actual allocation logic
    /// Currently, it only checks the parameters and returns success.
    pub fn sys_fallocate(&self, fd: usize, mode: i32, offset: isize, len: isize) -> SyscallResult {
        if offset < 0 || len <= 0 {
            error!("[sys_fallocate] negative offset or len");
            return Err(Errno::EINVAL);
        }
        if (offset + len) as usize > EXT4_MAX_FILE_SIZE {
            error!("[sys_fallocate] too big !!");
            return Err(Errno::EFBIG);
        }

        let mode = FallocFlags::from_bits(mode).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_fallocate] fd: {}, mode: {:?}, offset: {}, len: {}",
            fd, mode, offset, len
        );

        let file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        let inode = file.inode();

        if inode.file_type() == InodeMode::DIR {
            return Err(Errno::EISDIR);
        }
        if !file.meta().writable() {
            return Err(Errno::EBADF);
        }

        Ok(0)
    }

    pub async fn sys_mknodat(
        &self,
        fd: isize,
        path: usize,
        mode: usize,
        dev: u64,
    ) -> SyscallResult {
        let mode = InodeMode::from_bits(mode as u32).ok_or(Errno::EINVAL)?;
        let path = read_path(path)?;
        let searchflags = SearchFlags::AT_SYMLINK_NOFOLLOW;
        let (parent, name) = get_dentry_parent(self.task, fd, &path, &searchflags)?;
        info!(
            "[sys_mknodat] dirfd: {}, path: {:?}, mode: {:?}, dev: {}",
            fd, path, mode, dev
        );

        let dev_t = if dev < 0xffff {
            let major = (dev >> 8) as u32 & 0xff;
            let minor = (dev & 0xff) as u32;
            DevT::new_encode_dev(major, minor)
        } else {
            DevT::new(dev)
        };

        if mode.contains(InodeMode::FILE) {
            parent.create(name, mode).await?;
        } else {
            parent.mknodat_son(&name, dev_t, mode)?;
        }

        Ok(0)
    }

    // todo: implement the actual fadvise logic
    pub fn sys_fadvise64(
        &self,
        fd: usize,
        offset: usize,
        len: usize,
        advice: i32,
    ) -> SyscallResult {
        info!(
            "[sys_fadvise64] fd: {}, offset: {}, len: {}, advice: {}",
            fd, offset, len, advice
        );
        log::warn!("[sys_fadvise64] Unimplemented");
        match advice {
            0..=5 => {}
            _ => return Err(Errno::EINVAL),
        }
        let _file = self.task.fd_table().get(fd).ok_or(Errno::EBADF)?;
        Ok(0)
    }
}

#[inline(always)]
fn read_path(raw: usize) -> SysResult<String> {
    let ptr = UserPtr::<u8>::new(raw);
    ptr.get_string_from_ptr()
}

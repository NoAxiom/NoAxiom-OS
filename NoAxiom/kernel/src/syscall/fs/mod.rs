use super::{Syscall, SyscallResult};
use crate::{
    constant::fs::{STD_ERR, STD_IN, STD_OUT},
    fs::path::{check_path, Path},
    mm::user_ptr::UserPtr,
    nix::result::Errno,
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
        let ptr = UserPtr::<&[u8]>::new(buf as usize);
        let data_cloned = ptr.as_ref_mut(); // this might trigger pagefault
        *data_cloned = cwd.as_string().as_bytes();

        Ok(buf as isize)
    }

    /// Create a pipe
    pub fn sys_pipe2(&self, pipe: *mut i32, flag: usize) -> SyscallResult {
        info!("[sys_pipe2] not implemented");
        Err(Errno::ENOSYS)
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
    pub fn sys_chdir(&self, path: *const u8) -> SyscallResult {
        info!("[sys_chdir] path: {:?}", path);
        let ptr = UserPtr::<u8>::new(path as usize);
        let path = get_string_from_ptr(&ptr);

        if !check_path(&path) {
            return Err(Errno::ENOENT);
        }

        let mut pcb_guard = self.task.pcb();
        pcb_guard.cwd = Path::from(path.clone());
        Ok(0)
    }

    /// Read data from a file descriptor
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
}

fn not_stdio(fd: usize) -> bool {
    fd != STD_IN && fd != STD_OUT && fd != STD_ERR
}

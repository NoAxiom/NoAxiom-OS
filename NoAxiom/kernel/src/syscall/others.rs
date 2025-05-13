use super::{Syscall, SyscallResult};
use crate::{mm::user_ptr::UserPtr, sched::utils::yield_now, utils::random_fill};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield(&self) -> SyscallResult {
        trace!("sys_yield");
        yield_now().await;
        Ok(0)
    }

    /// get a random number
    pub async fn sys_getrandom(&self, buf: usize, buflen: usize, _flags: usize) -> SyscallResult {
        info!("[sys_getrandom] buf: {:#x}, buflen: {}", buf, buflen);
        let user_ptr = UserPtr::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;
        random_fill(buf_slice);
        Ok(buflen as isize)
    }
}

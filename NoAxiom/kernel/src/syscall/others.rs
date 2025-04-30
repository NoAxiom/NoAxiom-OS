use super::{Syscall, SyscallResult};
use crate::{mm::user_ptr::UserPtr, sched::utils::yield_now, utils::random};

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

        let mut remaining = buf_slice.len();
        let mut offset = 0;

        while remaining > 0 {
            let rand_bytes = random().to_le_bytes();
            let chunk_size = remaining.min(4);
            buf_slice[offset..offset + chunk_size].copy_from_slice(&rand_bytes[..chunk_size]);
            remaining -= chunk_size;
            offset += chunk_size;
        }

        Ok(buflen as isize)
    }
}

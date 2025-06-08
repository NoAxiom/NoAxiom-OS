use super::{Syscall, SyscallResult};
use crate::{mm::user_ptr::UserPtr, utils::random_fill};

impl Syscall<'_> {
    /// get a random number
    pub async fn sys_getrandom(&self, buf: usize, buflen: usize, _flags: usize) -> SyscallResult {
        info!("[sys_getrandom] buf: {:#x}, buflen: {}", buf, buflen);
        let user_ptr = UserPtr::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;
        random_fill(buf_slice);
        Ok(buflen as isize)
    }

    /// systemshutdown
    pub fn sys_systemshutdown() -> ! {
        #[cfg(feature = "debug_sig")]
        {
            println!(
                "[kernel] press any key to shutdown, system time: {:?}",
                crate::time::gettime::get_time_duration()
            );
            while platform::getchar() as i8 == -1 {}
        }
        platform::shutdown()
    }
}

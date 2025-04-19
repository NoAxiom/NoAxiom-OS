use super::{Syscall, SyscallResult};
use crate::{include::info::Utsname, mm::user_ptr::UserPtr};

impl Syscall<'_> {
    /// Get system UTS name
    pub fn sys_uname(buf: usize) -> SyscallResult {
        let buf = UserPtr::<Utsname>::new(buf);
        let res = Utsname::get();
        buf.write(res);
        Ok(0)
    }

    /// Get system log
    pub async fn sys_syslog(_log_type: usize, buf: usize, len: usize) -> SyscallResult {
        let user_ptr = UserPtr::<u8>::new(buf);
        user_ptr.as_slice_mut_checked(len).await?;
        warn!("[sys_log] just check buf");
        Ok(0)
    }
}

use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    include::info::{SyslogAction, Utsname},
    mm::user_ptr::UserPtr,
};

impl Syscall<'_> {
    /// Get system UTS name
    pub fn sys_uname(buf: usize) -> SyscallResult {
        let buf = UserPtr::<Utsname>::new(buf);
        let res = Utsname::get();
        buf.write(res);
        Ok(0)
    }

    /// Get system log
    pub async fn sys_syslog(log_type: u32, buf: usize, len: usize) -> SyscallResult {
        let user_ptr = UserPtr::<u8>::new(buf);
        let log_type = SyslogAction::from_repr(log_type).ok_or(Errno::EINVAL)?;
        match log_type {
            SyslogAction::OPEN | SyslogAction::CLOSE => Ok(0),
            SyslogAction::READ | SyslogAction::ReadAll | SyslogAction::ReadClear => {
                user_ptr.as_slice_mut_checked(len).await?;
                Ok(0)
            }
            SyslogAction::Unknown => Err(Errno::EINVAL),
            _ => Ok(0),
        }
    }
}

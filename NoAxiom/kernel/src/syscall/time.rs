use super::{Syscall, SyscallResult};
use crate::{
    mm::user_ptr::UserPtr,
    time::{
        gettime::{get_time_ms, get_timeval},
        time_spec::TimeSpec,
        time_val::TimeVal,
        tms::TMS,
    },
};

impl Syscall<'_> {
    pub fn sys_times(&self, tms: usize) -> SyscallResult {
        let tms = UserPtr::<TMS>::new(tms);
        let res = self.task.tcb().time_stat.into();
        tms.write(res);
        Ok(0)
    }
    pub fn sys_gettimeofday(buf: usize) -> SyscallResult {
        if buf == 0 {
            return Ok(get_time_ms() as isize);
        }
        let buf = UserPtr::<TimeVal>::new(buf);
        let timeval = get_timeval();
        buf.write(timeval);
        Ok(0)
    }
    pub async fn sys_nanosleep(&self, buf: usize) -> SyscallResult {
        let buf = UserPtr::<TimeSpec>::new(buf);
        let time_spec = buf.read();
        self.task.sleep(time_spec.into()).await;
        Ok(0)
    }
}

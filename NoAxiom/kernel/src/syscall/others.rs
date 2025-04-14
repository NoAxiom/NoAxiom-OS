use super::{Syscall, SyscallResult};
use crate::{
    include::info::Utsname,
    mm::user_ptr::UserPtr,
    sched::utils::yield_now,
    time::{
        gettime::{get_time_ms, get_time_ns, get_time_us, get_timeval},
        time_val::{TimeSpec, TimeVal},
        tms::TMS,
    },
};

impl Syscall<'_> {
    /// yield current task
    pub async fn sys_yield(&self) -> SyscallResult {
        trace!("sys_yield");
        yield_now().await;
        Ok(0)
    }

    pub fn sys_times(tms: usize) -> SyscallResult {
        let tms = UserPtr::<TMS>::new(tms);
        let sec = get_time_us() as isize;
        let res = TMS {
            tms_utime: sec,
            tms_stime: sec,
            tms_cutime: sec,
            tms_cstime: sec,
        };
        tms.write(res);
        Ok(0)
    }

    pub fn sys_uname(buf: usize) -> SyscallResult {
        let buf = UserPtr::<Utsname>::new(buf);
        let res = Utsname::get();
        buf.write(res);
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
        self.task.sleep(time_spec.into_ticks()).await;
        Ok(0)
    }

    /// get a random number
    pub async fn sys_getrandom(&self, buf: usize, buflen: usize, _flags: usize) -> SyscallResult {
        info!("[sys_getrandom] buf: {:#x}, buflen: {}", buf, buflen);
        let user_ptr = UserPtr::<u8>::new(buf);
        let buf_slice = user_ptr.as_slice_mut_checked(buflen).await?;

        let mut remaining = buf_slice.len();
        let mut offset = 0;

        while remaining > 0 {
            let rand = get_time_ns(); // use time as rand
            let rand_bytes = rand.to_le_bytes();
            let chunk_size = remaining.min(4);
            buf_slice[offset..offset + chunk_size].copy_from_slice(&rand_bytes[..chunk_size]);
            remaining -= chunk_size;
            offset += chunk_size;
        }

        Ok(buflen as isize)
    }
}

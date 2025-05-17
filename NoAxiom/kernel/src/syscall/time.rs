use core::time::Duration;

use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    mm::user_ptr::UserPtr,
    time::{
        clock::{ClockId, CLOCK_MANAGER},
        gettime::{get_time_duration, get_time_ms, get_timeval},
        time_info::TMS,
        time_spec::TimeSpec,
        time_val::TimeVal,
        timeout::sleep_now,
    },
};

impl Syscall<'_> {
    pub async fn sys_times(&self, tms: usize) -> SyscallResult {
        let tms = UserPtr::<TMS>::new(tms);
        let res = self.task.tcb().time_stat.into_tms();
        tms.write(res).await?;
        Ok(0)
    }
    pub async fn sys_gettimeofday(tv: usize) -> SyscallResult {
        if tv == 0 {
            return Ok(get_time_ms() as isize);
        }
        let buf = UserPtr::<TimeVal>::new(tv);
        let timeval = get_timeval();
        buf.write(timeval).await?;
        Ok(0)
    }
    pub async fn sys_nanosleep(&self, buf: usize, remain: usize) -> SyscallResult {
        let ts = UserPtr::<TimeSpec>::new(buf);
        let remain = UserPtr::<TimeSpec>::new(remain);
        if ts.is_null() {
            return Err(Errno::EINVAL);
        }
        let time_spec = ts.read().await?;
        let remain_time = sleep_now(time_spec.into()).await;
        if !remain.is_null() {
            if remain_time > Duration::ZERO {
                remain.write(remain_time.into()).await?;
                error!(
                    "[sys_nanosleep] sleep interrupted, remain time: {:?}",
                    remain_time
                );
                return Err(Errno::EINTR);
            } else {
                return Ok(0);
            }
        }
        Ok(0)
    }
    pub async fn sys_clock_gettime(&self, clockid: usize, tp: usize) -> SyscallResult {
        let ts = UserPtr::<TimeSpec>::from(tp);
        let clockid = ClockId::from_repr(clockid).ok_or(Errno::EINVAL)?;
        info!(
            "[sys_clock_gettime] clock_id: {:?}, ts_addr: {:#x}",
            clockid,
            ts.addr(),
        );
        use ClockId::*;
        let time = match clockid {
            CLOCK_PROCESS_CPUTIME_ID => {
                let task = self.task;
                let mut cpu_time = Duration::ZERO;
                for (_tid, task) in task.thread_group().0.iter() {
                    if let Some(task) = task.upgrade() {
                        let tcb = task.tcb();
                        cpu_time += tcb.time_stat.cpu_time();
                    }
                }
                trace!("[sys_clock_gettime] get process cpu time: {:?}", cpu_time);
                cpu_time
            }
            CLOCK_THREAD_CPUTIME_ID => {
                let cpu_time = self.task.tcb().time_stat.cpu_time();
                trace!("[sys_clock_gettime] get process cpu time: {:?}", cpu_time);
                cpu_time
            }
            _ => match CLOCK_MANAGER.lock().0.get(&clockid) {
                Some(clock) => {
                    let dev_time = get_time_duration();
                    let clock_time = dev_time + *clock;
                    trace!("[sys_clock_gettime] get time {:?}", clock_time);
                    clock_time
                }
                None => {
                    error!("[sys_clock_gettime] Cannot find the clock: {:?}", clockid);
                    return Err(Errno::EINVAL);
                }
            },
        };
        ts.write(TimeSpec::from(time)).await?;
        Ok(0)
    }
    pub async fn sys_clock_nanosleep(
        &self,
        _clock_id: usize,
        flags: usize,
        request: usize,
        remain: usize,
    ) -> SyscallResult {
        pub const TIMER_ABSTIME: usize = 1;
        let request = UserPtr::<TimeSpec>::new(request);
        let remain = UserPtr::<TimeSpec>::new(remain);
        let request = Duration::from(request.read().await?);
        let current = get_time_duration();
        let remain_time = if flags == TIMER_ABSTIME {
            if request < current {
                Duration::ZERO
            } else {
                sleep_now(request - current).await
            }
        } else {
            sleep_now(request).await
        };
        if !remain.is_null() {
            remain.write(remain_time.into()).await?;
        }
        Ok(0)
    }

    pub async fn sys_clock_getres(&self, _clockid: usize, res: usize) -> SyscallResult {
        let res = UserPtr::<TimeSpec>::new(res);
        if res.is_null() {
            return Ok(0);
        }
        let value = TimeSpec {
            tv_sec: 0,
            tv_nsec: 1,
        };
        res.write(value).await?;
        Ok(0)
    }
}

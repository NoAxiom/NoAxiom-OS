use alloc::boxed::Box;
use core::time::Duration;

use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    include::time::{ITimerType, ITimerVal, TimeSpec, TimeVal, ITIMER_COUNT, TMS},
    mm::user_ptr::UserPtr,
    return_errno,
    sched::utils::realtime,
    time::{
        clock::{ClockId, CLOCK_MANAGER},
        gettime::{get_time_duration, get_time_ms, get_timeval},
        timeout::sleep_now,
        timer::{ITimer, ITimerReal, Timer, TIMER_MANAGER},
    },
};

impl Syscall<'_> {
    pub async fn sys_times(&self, tms: usize) -> SyscallResult {
        let tms = UserPtr::<TMS>::new(tms);
        let res = self.task.time_stat().into_tms();
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
        let remain_time = realtime(self.task, sleep_now(time_spec.into())).await;
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
        trace!(
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
                        cpu_time += task.time_stat().cpu_time();
                    }
                }
                trace!("[sys_clock_gettime] get process cpu time: {:?}", cpu_time);
                cpu_time
            }
            CLOCK_THREAD_CPUTIME_ID => {
                let cpu_time = self.task.time_stat().cpu_time();
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
        let remain_time = realtime(self.task, async move {
            if flags == TIMER_ABSTIME {
                if request < current {
                    Duration::ZERO
                } else {
                    sleep_now(request - current).await
                }
            } else {
                sleep_now(request).await
            }
        })
        .await;
        if !remain.is_null() {
            remain.write(remain_time.into()).await?;
        }
        Ok(0)
    }

    pub async fn sys_clock_getres(&self, clockid: isize, res: usize) -> SyscallResult {
        let res = UserPtr::<TimeSpec>::new(res);
        if clockid < 0 {
            return Err(Errno::EINVAL);
        }
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

    /// get interval timer
    pub async fn sys_getitimer(&self, which: usize, curr_value: usize) -> SyscallResult {
        let curr_value = UserPtr::<ITimerVal>::new(curr_value);
        if curr_value.is_null() {
            return Err(Errno::EFAULT);
        }
        if which >= ITIMER_COUNT {
            return Err(Errno::EINVAL);
        }
        let manager = self.task.itimer();
        let itimer = manager.get(which);
        let itimer_val = ITimerVal {
            it_interval: itimer.interval.into(),
            it_value: itimer.expire.saturating_sub(get_time_duration()).into(),
        };
        curr_value.write(itimer_val).await?;
        Ok(0)
    }

    /// set interval timer
    pub async fn sys_setitimer(
        &self,
        which: usize,
        new_value: usize,
        old_value: usize,
    ) -> SyscallResult {
        let new_value = UserPtr::<ITimerVal>::new(new_value);
        let old_value = UserPtr::<ITimerVal>::new(old_value);
        let itimer_type = ITimerType::from_repr(which).ok_or(Errno::EINVAL)?;
        let new_value = new_value.read().await?;
        let mut manager = self.task.itimer();
        let old_itimer = manager.get(which);
        match itimer_type {
            ITimerType::Real => {
                let old = old_itimer.into_itimer_val();
                let new_itimer = ITimer::register(&new_value);
                let timer_id = new_itimer.timer_id;
                manager.set(which, new_itimer);
                if !new_itimer.is_disarmed() {
                    let timer = Timer::new(
                        new_itimer.expire,
                        Box::new(ITimerReal::new(self.task, timer_id)),
                    );
                    TIMER_MANAGER.add_timer(timer);
                }
                old_value.try_write(old).await?;
            }
            ITimerType::Virtual => return_errno!(Errno::EINVAL, "ITIMER_VIRTUAL is unimplemented"),
            ITimerType::Prof => return_errno!(Errno::EINVAL, "ITIMER_PROF is unimplemented"),
        };
        Ok(0)
    }
}

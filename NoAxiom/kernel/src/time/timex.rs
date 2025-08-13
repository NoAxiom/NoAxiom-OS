use core::time::Duration;

use include::errno::{Errno, SyscallResult};
use ksync::mutex::SpinLock;

use crate::{
    constant::time::NSEC_PER_SEC,
    include::time::{LinuxTimex, TimeSpec, TimexModes},
    time::gettime::get_time_duration,
};

pub static LAST_TIMEX: SpinLock<Option<LinuxTimex>> = SpinLock::new(None);

pub fn adjtimex(timex: &mut LinuxTimex) -> SyscallResult {
    if timex.modes == 0x8000 {
        return Err(Errno::EINVAL);
    }
    let modes = TimexModes::from_bits(timex.modes).ok_or(Errno::EINVAL)?;

    if modes.contains(TimexModes::ADJ_SETOFFSET) {
        let mut delta = TimeSpec::default();
        delta.tv_sec = timex.time.sec;
        delta.tv_nsec = timex.time.usec;
        if modes.contains(TimexModes::ADJ_NANO) {
            delta.tv_nsec *= 1000;
        }
        adjtimex_inject(&delta)?;
    }
    if modes.contains(TimexModes::ADJ_TICK) {
        let limit = timex.tick;
        if limit < 9000 || limit > 11000 {
            return Err(Errno::EINVAL);
        }
    }

    Ok(0)
}

fn adjtimex_inject(delta: &TimeSpec) -> SyscallResult {
    if delta.tv_nsec >= NSEC_PER_SEC {
        return Err(Errno::EINVAL);
    }
    let wall_clock = TimeSpec::new_bare();
    let wall_duration = Duration::from(wall_clock);
    let new_time = TimeSpec {
        tv_sec: delta.tv_sec,
        tv_nsec: delta.tv_nsec,
    };
    let monotonic_clock = TimeSpec::from(get_time_duration());
    let monotonic_duration = Duration::from(monotonic_clock);
    let delta_duration = Duration::from(*delta);
    if wall_duration + delta_duration > monotonic_duration || !new_time.is_valid() {
        return Err(Errno::EINVAL);
    }
    Ok(0)
}

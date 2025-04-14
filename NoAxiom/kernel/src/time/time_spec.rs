use core::time::Duration;

/// Describes times in seconds and nanoseconds.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct TimeSpec {
    pub tv_sec: usize,
    pub tv_nsec: usize,
}

impl TimeSpec {
    pub fn into_ms(&self) -> usize {
        self.tv_sec * 1_000 + self.tv_nsec / 1_000_000
    }

    pub fn from_ms(ms: usize) -> Self {
        Self {
            tv_sec: ms / 1000,
            tv_nsec: (ms % 1000) * 1_000_000,
        }
    }

    pub fn is_valid(&self) -> bool {
        (self.tv_sec as isize >= 0)
            && (self.tv_nsec as isize >= 0)
            && (self.tv_nsec < 1_000_000_000)
    }
}

impl From<Duration> for TimeSpec {
    fn from(duration: Duration) -> Self {
        Self {
            tv_sec: duration.as_secs() as usize,
            tv_nsec: duration.subsec_nanos() as usize,
        }
    }
}

impl From<TimeSpec> for Duration {
    fn from(time_spec: TimeSpec) -> Self {
        Duration::new(time_spec.tv_sec as u64, time_spec.tv_nsec as u32)
    }
}

use riscv::register::time;
use sbi_rt::legacy::set_timer;

use super::RV64;
use crate::ArchTime;

/// clock frequency: 4MHz, ns per tick: 250ns
#[cfg(feature = "vf2")]
const FREQ: usize = 4000000;

/// clock frequency: 12.5MHz, ns per tick: 80ns
#[cfg(not(feature = "vf2"))]
const FREQ: usize = 12500000;

impl ArchTime for RV64 {
    fn time_init() {}
    fn get_freq() -> usize {
        FREQ
    }
    #[inline(always)]
    fn get_time() -> usize {
        time::read()
    }
    fn set_timer(interval: u64) {
        set_timer(time::read() as u64 + interval);
    }
}

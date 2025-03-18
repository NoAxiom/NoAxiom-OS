use riscv::register::time;
use sbi_rt::legacy::set_timer;

use super::RV64;
use crate::ArchTime;

impl ArchTime for RV64 {
    #[inline(always)]
    fn get_time() -> usize {
        time::read()
    }
    fn set_timer(interval: u64) {
        set_timer(time::read() as u64 + interval);
    }
}

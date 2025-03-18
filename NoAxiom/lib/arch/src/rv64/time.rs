use riscv::register::time;
use sbi_rt::legacy::set_timer;

use super::RV64;
use crate::ArchTime;

impl ArchTime for RV64 {
    #[inline(always)]
    fn get_time() -> usize {
        time::read()
    }
    fn set_timer(time_value: u64) {
        set_timer(time_value)
    }
}

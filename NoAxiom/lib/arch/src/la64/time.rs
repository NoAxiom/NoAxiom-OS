use core::arch::asm;

use loongArch64::{register::ticlr::clear_timer_interrupt, time::Time};

use super::LA64;
use crate::ArchTime;

impl ArchTime for LA64 {
    fn get_time() -> usize {
        Time::read()
    }
    fn set_timer(time_value: u64) {
        clear_timer_interrupt();
        let time = Time::read();
        let time_value = time.wrapping_add(time_value as usize);
        unsafe {
            asm!(
                "wrtie.d {},{}",
                in(reg) time_value,
                in(reg) 0,
            );
        }
    }
}

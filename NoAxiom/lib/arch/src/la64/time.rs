use loongArch64::{
    register::{
        tcfg::{set_en, set_init_val},
        ticlr::clear_timer_interrupt,
    },
    time::Time,
};

use super::LA64;
use crate::ArchTime;

impl ArchTime for LA64 {
    fn get_time() -> usize {
        Time::read()
    }
    fn set_timer(interval: u64) {
        set_init_val(interval as usize);
        clear_timer_interrupt();
        set_en(true);
    }
}

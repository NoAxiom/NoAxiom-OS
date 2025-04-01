use log::info;
use loongArch64::{
    asm::idle,
    register::{tcfg, ticlr},
    time::{get_timer_freq, Time},
};
use spin::Lazy;

use super::LA64;
use crate::ArchTime;

static FREQ: Lazy<usize> = Lazy::new(|| get_timer_freq());

pub fn time_init() {
    let ticks = ((*FREQ / 1000) + 3) & !3;
    tcfg::set_periodic(true); // set timer to one-shot mode
    tcfg::set_init_val(ticks); // set timer initial value
    tcfg::set_en(true); // enable timer
}

impl ArchTime for LA64 {
    fn time_init() {
        time_init();
    }
    fn get_freq() -> usize {
        *FREQ
    }
    fn get_time() -> usize {
        Time::read()
    }
    fn set_timer(interval: u64) {
        ticlr::clear_timer_interrupt();
        tcfg::set_init_val(interval as usize);
        tcfg::set_en(true);
    }
}

use sbi_rt::{
    hart_start,
    legacy::{clear_ipi, console_getchar, console_putchar, shutdown},
    HartMask,
};

use super::RV64;
use crate::ArchSbi;

impl ArchSbi for RV64 {
    fn console_init() {}
    // write in console
    fn console_putchar(c: usize) {
        console_putchar(c);
    }
    // read in console
    fn console_getchar() -> usize {
        console_getchar()
    }
    // send ipi
    fn send_ipi(hartid: usize) {
        sbi_rt::send_ipi(HartMask::from_mask_base(0b1, hartid));
    }
    // clear ipi
    fn clear_ipi() {
        clear_ipi();
    }
    // shutdown
    fn shutdown() -> ! {
        shutdown()
    }
    // hart start
    fn hart_start(hartid: usize, start_addr: usize) {
        let x = hart_start(hartid, start_addr, 0);
        if x.is_err() {
            panic!("hart_start failed");
        }
    }
}

use sbi_rt::{hart_start, legacy::clear_ipi, HartMask};

use super::RV64;
use crate::ArchSbi;

impl ArchSbi for RV64 {
    // send ipi
    fn send_ipi(hartid: usize) {
        sbi_rt::send_ipi(HartMask::from_mask_base(0b1, hartid));
    }
    // clear ipi
    fn clear_ipi() {
        clear_ipi();
    }
    // hart start
    fn hart_start(hartid: usize, start_addr: usize) {
        let x = hart_start(hartid, start_addr, 0);
        if x.is_err() {
            panic!("hart_start failed");
        }
    }
}

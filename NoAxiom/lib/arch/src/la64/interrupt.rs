use log::warn;
use loongArch64::register::{crmd, estat};

use super::LA64;
use crate::ArchInt;

impl ArchInt for LA64 {
    fn is_interrupt_enabled() -> bool {
        let crmd = crmd::read();
        crmd.ie()
    }
    fn enable_global_interrupt() {
        crmd::set_ie(true);
    }
    fn disable_global_interrupt() {
        crmd::set_ie(false);
    }

    // not implemented
    fn disable_external_interrupt() {}
    fn disable_user_memory_access() {}

    // 8 hard interrupt in ESTAT.IS[9..2]
    fn enable_external_interrupt() {
        // now we only support external interrupt 2
        estat::set_sw(2, true);
    }

    // 2 soft interrupt in ESTAT.IS[1..0]
    fn enable_software_interrupt() {
        // now we only support software interrupt 0
        estat::set_sw(0, true);
    }

    fn enable_stimer_interrupt() {}
    fn enable_user_memory_access() {}

    fn is_external_interrupt_enabled() -> bool {
        let is = estat::read().is();
        let mut enabled = false;
        for i in 2..9 {
            if bit_field::BitField::get_bit(&is, i) {
                enabled = true;
                break;
            }
        }
        enabled
    }
}

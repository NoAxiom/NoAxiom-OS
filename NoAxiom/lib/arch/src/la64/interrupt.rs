use loongArch64::{
    ipi::send_ipi_single,
    register::{
        crmd,
        ecfg::{self, LineBasedInterrupt},
    },
};

use super::LA64;
use crate::ArchInt;

#[inline]
pub(crate) fn is_interrupt_enabled() -> bool {
    crmd::read().ie()
}

#[inline]
pub(crate) fn enable_interrupt() {
    crmd::set_ie(true);
}

#[inline]
pub(crate) fn disable_interrupt() {
    crmd::set_ie(false);
}

#[inline]
pub(crate) fn enable_external_interrupt() {
    ecfg::set_lie(LineBasedInterrupt::HWI0);
}

#[inline]
pub(crate) fn enable_timer_interrupt() {
    ecfg::set_lie(LineBasedInterrupt::TIMER);
}

#[inline]
pub(crate) fn enable_software_interrupt() {
    ecfg::set_lie(LineBasedInterrupt::SWI0 | LineBasedInterrupt::SWI1);
}

pub(crate) fn interrupt_init() {
    let inter = LineBasedInterrupt::TIMER;
    ecfg::set_lie(inter);
}

// fake impl for 2k1000
// impl ArchInt for LA64 {
//     fn is_interrupt_enabled() -> bool {
//         true
//     }
//     fn enable_interrupt() {}
//     fn disable_interrupt() {}
//     fn disable_external_interrupt() {}
//     fn enable_external_interrupt() {}
//     fn enable_software_interrupt() {}
//     fn enable_timer_interrupt() {
//         enable_timer_interrupt();
//     }
//     fn is_external_interrupt_enabled() -> bool {
//         true
//     }
//     // user memory access is riscv specific
//     fn enable_user_memory_access() {}
//     fn disable_user_memory_access() {}
//     // ipi
//     fn send_ipi(hartid: usize) {
//         send_ipi_single(hartid, 1);
//     }
//     fn clear_ipi() {}
// }

impl ArchInt for LA64 {
    fn is_interrupt_enabled() -> bool {
        is_interrupt_enabled()
    }
    fn enable_interrupt() {
        enable_interrupt();
    }
    fn disable_interrupt() {
        disable_interrupt();
    }
    fn disable_external_interrupt() {
        // unimplemented!()
    }
    // 8 hard interrupt in ESTAT.IS[9..2]
    fn enable_external_interrupt() {
        // enable_external_interrupt();
    }
    // 2 soft interrupt in ESTAT.IS[1..0]
    fn enable_software_interrupt() {
        // enable_software_interrupt();
    }
    fn enable_timer_interrupt() {
        enable_timer_interrupt();
    }
    fn is_external_interrupt_enabled() -> bool {
        // let lie = ecfg::read().lie();
        // const MASK: usize = ((1 << 8) - 1) << 2;
        // lie.bits() & MASK != 0
        true
    }
    // user memory access is riscv specific
    fn enable_user_memory_access() {}
    fn disable_user_memory_access() {}
    // ipi
    fn send_ipi(hartid: usize) {
        send_ipi_single(hartid, 1);
    }
    fn clear_ipi() {
        unimplemented!()
    }
}

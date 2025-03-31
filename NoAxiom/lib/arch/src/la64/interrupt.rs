use loongArch64::register::{crmd, estat};

use super::LA64;
use crate::ArchInt;

#[inline]
pub(crate) fn is_interrupt_enabled() -> bool {
    let crmd = crmd::read();
    crmd.ie()
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
pub(crate) fn set_external_interrupt(value: bool) {
    // estat::set_sw(10, true); // pmi
    for i in 2..9 {
        estat::set_sw(i, value); // HWI0-HWI7
    }
    estat::set_sw(12, value); // ipi
}

#[inline]
pub(crate) fn enable_timer_interrupt() {
    estat::set_sw(11, true); // timer
}

#[inline]
pub(crate) fn enable_software_interrupt() {
    estat::set_sw(0, true); // SWI0
    estat::set_sw(1, true); // SWI1
}

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
        // todo: not checked, might be wrong
        set_external_interrupt(false);
    }
    // 8 hard interrupt in ESTAT.IS[9..2]
    fn enable_external_interrupt() {
        set_external_interrupt(true);
    }
    // 2 soft interrupt in ESTAT.IS[1..0]
    fn enable_software_interrupt() {
        enable_software_interrupt();
    }
    fn enable_timer_interrupt() {
        enable_timer_interrupt();
    }
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
    // user memory access is riscv specific
    fn enable_user_memory_access() {}
    fn disable_user_memory_access() {}
}

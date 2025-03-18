use loongArch64::register::crmd;

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
    fn enable_external_interrupt() {}
    fn enable_software_interrupt() {}
    fn enable_stimer_interrupt() {}
    fn enable_user_memory_access() {}
    fn is_external_interrupt_enabled() -> bool {
        true
    }
}

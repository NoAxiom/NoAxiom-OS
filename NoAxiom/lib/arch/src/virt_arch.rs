//! trait bound list:
//! - [`FullVirtArch`] impl all traits below
//! - [`ArchInt`]
//! - [`ArchAsm`]
//! - [`ArchSbi`]
//! - [`ArchType`]
//! - [`ArchTrap`]
//! - [`ArchTime`]
//! - [`ArchInfo`]
//! - [`ArchMemory`]

/// interrupt related arch trait
pub trait ArchInt {
    // global interrupt
    fn is_interrupt_enabled() -> bool {
        unimplemented!("is_interrupt_enabled")
    }
    fn disable_global_interrupt() {
        unimplemented!("disable_global_interrupt")
    }
    fn enable_global_interrupt() {
        unimplemented!("enable_global_interrupt")
    }

    // external interrupt
    fn enable_external_interrupt() {
        unimplemented!("enable_external_interrupt")
    }
    fn disable_external_interrupt() {
        unimplemented!("disable_external_interrupt")
    }
    fn is_external_interrupt_enabled() -> bool {
        unimplemented!("is_external_interrupt_enabled")
    }

    // soft / timer interrupt
    fn enable_software_interrupt() {
        unimplemented!("enable_software_interrupt")
    }
    fn enable_stimer_interrupt() {
        unimplemented!("enable_stimer_interrupt")
    }

    // user memory access
    fn enable_user_memory_access() {
        unimplemented!("enable_user_memory_access")
    }
    fn disable_user_memory_access() {
        unimplemented!("disable_user_memory_access")
    }
}

/// hart related arch trait
pub trait ArchAsm {
    // get hartid
    fn get_hartid() -> usize {
        unimplemented!("get_hartid")
    }
    fn set_idle() {
        unimplemented!("set_idle")
    }
    fn current_pc() -> usize {
        unimplemented!("current_pc")
    }
}

/// basic arch types defination
pub trait ArchType {
    type Trap;
    type Interrupt;
    type Exception;
    type TrapContext;
}

/// sbi related arch trait
pub trait ArchSbi {
    fn console_putchar(_c: usize) {
        unimplemented!("console_putchar")
    }
    fn console_getchar() -> usize {
        unimplemented!("console_getchar")
    }
    fn send_ipi(_hartid: usize) {
        unimplemented!("send_ipi")
    }
    fn clear_ipi() {
        unimplemented!("clear_ipi")
    }
    fn shutdown() -> ! {
        unimplemented!("shutdown")
    }
    fn hart_start(_hartid: usize, _start_addr: usize, _opaque: usize) {
        unimplemented!("hart_start")
    }
}

/// memory management arch trait
pub trait ArchMemory {
    fn tlb_flush() {
        unimplemented!("tlb_flush")
    }
    fn update_pagetable(_bits: usize) {
        unimplemented!("update_pagetable")
    }
}

/// trap related arch trait
pub trait ArchTrap: ArchType {
    fn set_trap_entry(_addr: usize) {
        unimplemented!("set_trap_entry")
    }
    fn read_trap_cause() -> <Self as ArchType>::Trap {
        unimplemented!("read_trap_cause")
    }
    fn read_trap_value() -> usize {
        unimplemented!("read_trap_value")
    }
    fn read_trap_pc() -> usize {
        unimplemented!("read_trap_pc")
    }
    fn set_kernel_trap_entry() {
        unimplemented!("set_kernel_trap_entry")
    }
    fn set_user_trap_entry() {
        unimplemented!("set_user_trap_entry")
    }
    fn trap_init() {
        unimplemented!("trap_init")
    }
    fn trap_restore(_cx: &mut <Self as ArchType>::TrapContext) {
        unimplemented!("trap_restore")
    }
}

/// time related arch trait
pub trait ArchTime {
    fn get_time() -> usize {
        unimplemented!("get_time")
    }
    fn set_timer(_time_value: u64) -> usize {
        unimplemented!("set_timer")
    }
}

/// arch info
pub trait ArchInfo {
    const ARCH_NAME: &'static str = "unknown";
}

/// full arch trait
pub trait FullVirtArch:
    ArchInt + ArchAsm + ArchSbi + ArchType + ArchTrap + ArchTime + ArchInfo
{
    // should impl all traits above
}

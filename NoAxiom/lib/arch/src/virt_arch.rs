/// interrupt related arch trait
pub trait ArchInt {
    // global interrupt
    fn is_interrupt_enabled() -> bool;
    fn disable_global_interrupt();
    fn enable_global_interrupt();

    // external interrupt
    fn enable_external_interrupt();
    fn disable_external_interrupt();
    fn is_external_interrupt_enabled() -> bool;

    // soft / timer interrupt
    fn enable_software_interrupt();
    fn enable_stimer_interrupt();

    // user memory access
    fn enable_user_memory_access();
    fn disable_user_memory_access();
}

/// hart related arch trait
pub trait ArchAsm {
    // get hartid
    fn get_hartid() -> usize;
    fn set_idle();
    fn current_pc() -> usize;
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
    fn console_putchar(c: usize);
    fn console_getchar() -> usize;
    fn send_ipi(hartid: usize);
    fn clear_ipi();
    fn shutdown() -> !;
    fn hart_start(hartid: usize, start_addr: usize, opaque: usize);
}

/// memory management arch trait
pub trait ArchMemory {
    fn tlb_flush();
    fn update_pagetable(bits: usize);
}

/// trap related arch trait
pub trait ArchTrap: ArchType {
    fn set_trap_entry(addr: usize);
    fn read_trap_cause() -> <Self as ArchType>::Trap;
    fn read_trap_value() -> usize;
    fn read_trap_pc() -> usize;
}

/// time related arch trait
pub trait ArchTime {
    fn get_time() -> usize;
    fn set_timer(time_value: u64) -> usize;
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

pub trait VirtArch {
    type Trap;
    type Interrupt;
    type Exception;
    type TrapContext;

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

    // get hartid
    fn get_hartid() -> usize;

    // sbi
    fn console_putchar(c: usize);
    fn console_getchar() -> usize;
    fn send_ipi(hartid: usize);
    fn clear_ipi();
    fn shutdown() -> !;
    fn hart_start(hartid: usize, start_addr: usize, opaque: usize);

    // memory
    fn tlb_flush();
    fn update_pagetable(bits: usize);

    // time
    fn get_time() -> usize;
    fn set_timer(time_value: u64) -> usize;

    // trap
    fn set_trap_entry(addr: usize);
    fn read_trap_cause() -> Self::Trap;
    fn read_trap_value() -> usize;
    fn read_trap_pc() -> usize;
}

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
    fn enable_timer_interrupt();

    // user memory access
    fn enable_user_memory_access();
    fn disable_user_memory_access();
}

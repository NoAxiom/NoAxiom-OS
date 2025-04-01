/// sbi related arch trait
pub trait ArchSbi {
    fn console_putchar(_c: usize);
    fn console_getchar() -> usize;
    fn send_ipi(hartid: usize);
    fn clear_ipi();
    fn shutdown() -> !;
    fn hart_start(hartid: usize, start_addr: usize);
}

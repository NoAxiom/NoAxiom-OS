/// sbi related arch trait
pub trait ArchSbi {
    fn console_putchar(_c: usize);
    fn console_getchar() -> usize;
    fn send_ipi(_hartid: usize);
    fn clear_ipi();
    fn shutdown() -> !;
    fn hart_start(_hartid: usize, _start_addr: usize, _opaque: usize);
}
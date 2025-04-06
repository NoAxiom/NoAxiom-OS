/// sbi related arch trait
pub trait ArchSbi {
    fn send_ipi(hartid: usize);
    fn clear_ipi();
    fn hart_start(hartid: usize, start_addr: usize);
}

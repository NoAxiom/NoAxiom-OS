pub trait ArchBoot {
    fn arch_init();
    fn hart_start(hartid: usize, start_addr: usize);
}

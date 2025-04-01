/// time related arch trait
pub trait ArchTime {
    fn time_init();
    fn get_freq() -> usize;
    fn get_time() -> usize;
    fn set_timer(interval: u64);
}
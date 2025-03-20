/// time related arch trait
pub trait ArchTime {
    fn get_time() -> usize;
    fn set_timer(interval: u64);
}
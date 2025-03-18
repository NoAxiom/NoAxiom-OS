/// time related arch trait
pub trait ArchTime {
    fn get_time() -> usize;
    fn set_timer(_time_value: u64);
}
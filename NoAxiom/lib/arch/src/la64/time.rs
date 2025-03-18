use super::LA64;
use crate::ArchTime;

impl ArchTime for LA64 {
    fn get_time() -> usize {
        unimplemented!()
    }
    fn set_timer(_time_value: u64) -> usize {
        0
    }
}

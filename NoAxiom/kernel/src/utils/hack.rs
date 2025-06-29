pub static mut IS_LTP: bool = false;

pub fn set_is_ltp(is_ltp: bool) {
    unsafe {
        IS_LTP = is_ltp;
    }
}
pub fn is_ltp() -> bool {
    unsafe { IS_LTP }
}
pub fn switch_into_ltp() {
    set_is_ltp(true);
}
pub fn switch_outof_ltp() {
    set_is_ltp(false);
}

use alloc::{string::String, vec::Vec};

use crate::mm::user_ptr::UserPtr;

#[inline(always)]
#[allow(unused)]
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn get_string_from_ptr(ptr: &UserPtr<u8>) -> String {
    let slice = ptr.clone_as_vec_until(|&c: &u8| c == 0);
    let res = String::from_utf8(Vec::from(slice)).unwrap();
    trace!("get_string_from_ptr: {}", res);
    res
}

#[inline(always)]
/// Generate a **Random** number at an extremely efficient way
pub fn random() -> usize {
    static mut SEED: usize = 253496567482;
    unsafe {
        SEED = SEED * 1103515245 + 12345;
        SEED
    }
}

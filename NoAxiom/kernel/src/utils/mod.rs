//! utility functions

mod crossover;
// pub mod event;
pub mod futures;

use alloc::{string::String, vec::Vec};

use crossover::{Crossover, CrossoverManager};

use crate::mm::user_ptr::UserPtr;

pub fn reverse<T: Clone>(vec: &Vec<T>) -> Vec<T> {
    let mut res = vec.clone();
    res.reverse();
    res
}

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

#[allow(unused)]
/// Execute a function every `interval` times
pub fn intermit(f: impl FnOnce()) {
    let interval = 89102;
    let id = &f as *const _ as usize;
    let mut guard = CrossoverManager.lock();
    let crossover = guard.entry(id).or_insert(Crossover::new(interval));
    if crossover.trigger() {
        f();
    }
}

/// Generate a **Random** number at an **EXTREMELY** efficient way
pub fn random() -> u64 {
    static mut SEED: u64 = 253496567482;
    unsafe {
        SEED = SEED * 1103515245 + 12345;
        SEED
    }
}

#[inline(always)]
pub fn random_fill(buf: &mut [u8]) -> usize {
    for i in 0..buf.len() {
        buf[i] = (random() & 0xFF) as u8;
    }
    buf.len()
}

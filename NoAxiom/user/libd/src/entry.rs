#![allow(unused)]
use alloc::{vec, vec::Vec};
use core::arch::asm;

use crate::{heap, syscall::exit};

#[linkage = "weak"]
#[no_mangle]
pub fn main(_: usize, _: &[&str]) -> isize {
    panic!("Cannot find main!");
}

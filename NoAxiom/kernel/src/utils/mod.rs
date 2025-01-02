//! utility functions

use alloc::{string::String, vec::Vec};

use crate::{
    config::mm::{KERNEL_ADDR_OFFSET, KERNEL_PAGENUM_MASK},
    mm::user_ptr::UserPtr,
};

/// signed extend for number without 64/32 bits width
#[inline(always)]
pub fn signed_extend(num: usize, width: usize) -> usize {
    if num & (1 << (width - 1)) != 0 {
        num | (!((1 << width) - 1))
    } else {
        num
    }
}

/// translate a raw usize type kernel virt address into phys address
#[inline(always)]
pub fn kernel_va_to_pa(virt: usize) -> usize {
    assert!(
        (virt & KERNEL_ADDR_OFFSET) == KERNEL_ADDR_OFFSET,
        "invalid kernel virt address"
    );
    virt & !KERNEL_ADDR_OFFSET
}

/// translate a raw usize type kernel phys address into virt address
#[inline(always)]
pub fn kernel_pa_to_va(phys: usize) -> usize {
    phys | KERNEL_ADDR_OFFSET
}

/// get current pc
#[inline(always)]
#[allow(unused)]
pub fn current_pc() -> usize {
    let pc: usize;
    unsafe { core::arch::asm!("auipc {}, 0", out(reg) pc) }
    pc
}

#[inline(always)]
pub fn kernel_vpn_to_ppn(vpn: usize) -> usize {
    vpn & !KERNEL_PAGENUM_MASK
}

pub fn reverse<T: Clone>(vec: &Vec<T>) -> Vec<T> {
    let mut res = vec.clone();
    res.reverse();
    res
}

#[inline(always)]
pub fn div_ceil(dividend: usize, divisor: usize) -> usize {
    (dividend + divisor - 1) / divisor
}

#[inline(always)]
pub fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn get_string_from_ptr(ptr: &mut UserPtr<u8>) -> String {
    let mut res = String::new();
    loop {
        let ch = *ptr.as_ref() as char;
        if ch == '\0' {
            break;
        }
        res.push(ch);
        ptr.inc_and_check(1);
    }
    res
}

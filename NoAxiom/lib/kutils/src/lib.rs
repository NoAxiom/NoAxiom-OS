#![no_std]

#[inline(always)]
pub fn get_hartid() -> usize {
    let hartid: usize;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
    hartid
}
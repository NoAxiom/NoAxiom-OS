#[inline(always)]
pub fn noaxiom_get_hartid() -> usize {
    let hartid: usize;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
    hartid
}

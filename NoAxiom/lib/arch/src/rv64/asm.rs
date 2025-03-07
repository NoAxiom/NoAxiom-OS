#[inline(always)]
pub fn get_hartid() -> usize {
    let hartid: usize;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) hartid) }
    hartid
}

#[inline(always)]
pub fn set_idle() {
    unsafe {
        riscv::asm::wfi();
    }
}

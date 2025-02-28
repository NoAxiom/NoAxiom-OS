use sbi_rt::HartMask;

#[inline(always)]
pub fn trigger_ipi(hart_id: usize) {
    sbi_rt::send_ipi(HartMask::from_mask_base(1 << hart_id, 0));
}

#[inline(always)]
pub fn console_putchar(c: usize) {
    sbi_rt::legacy::console_putchar(c);
}

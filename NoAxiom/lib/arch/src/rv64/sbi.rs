use sbi_rt::HartMask;

#[inline(always)]
pub fn trigger_ipi(hart_id: usize) {
    sbi_rt::send_ipi(HartMask::from_mask_base(1 << hart_id, 0));
}

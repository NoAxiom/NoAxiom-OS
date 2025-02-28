pub use riscv::register::*;

#[inline(always)]
pub fn address_translation_update(bits: usize) {
    satp::write(bits);
}

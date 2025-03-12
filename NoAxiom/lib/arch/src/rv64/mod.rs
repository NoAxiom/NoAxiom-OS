mod asm;
mod boot;
mod context;
mod interrupt;
mod memory;
mod other;
mod sbi;
mod time;
mod trap;
mod types;

pub struct RV64;

impl crate::FullVirtArch for RV64 {}

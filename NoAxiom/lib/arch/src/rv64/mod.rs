mod asm;
mod boot;
mod context;
mod interrupt;
mod memory;
mod other;
mod platform;
mod sbi;
mod time;
mod trap;

pub struct RV64;
pub use boot::{_entry, _entry_other_hart};

impl crate::FullVirtArch for RV64 {}

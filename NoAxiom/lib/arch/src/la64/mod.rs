mod asm;
mod boot;
mod context;
mod interrupt;
mod memory;
mod other;
mod sbi;
mod time;
mod trap;

pub use boot::{_entry, _entry_other_hart};

pub struct LA64;
impl crate::FullVirtArch for LA64 {}

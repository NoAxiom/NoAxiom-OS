mod asm;
mod boot;
mod context;
mod info;
mod interrupt;
mod memory;
mod time;
mod trap;

pub struct RV64;
pub use boot::{_entry, _entry_other_hart};

impl crate::ArchFull for RV64 {}

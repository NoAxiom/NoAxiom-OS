mod asm;
mod boot;
mod context;
mod interrupt;
mod memory;
mod sbi;
mod time;
mod trap;

pub struct RV64;
pub use boot::{_entry, _entry_other_hart};

impl crate::ArchFull for RV64 {
    const ARCH_NAME: &'static str = "riscv64";
}

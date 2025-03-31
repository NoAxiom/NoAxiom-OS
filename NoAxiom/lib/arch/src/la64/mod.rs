mod asm;
mod boot;
mod context;
mod interrupt;
mod memory;
mod poly;
mod sbi;
mod time;
mod tlb;
mod trap;
mod unaligned;

pub use boot::{_entry, _entry_other_hart};
pub mod la_libc_import;

pub struct LA64;
impl crate::ArchFull for LA64 {
    const ARCH_NAME: &'static str = "loongarch64";
}

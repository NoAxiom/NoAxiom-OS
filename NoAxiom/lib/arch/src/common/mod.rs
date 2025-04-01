mod asm;
mod interrupt;
mod memory;
mod other;
mod platform;
mod sbi;
mod time;
mod trap;

pub use asm::*;
pub use interrupt::*;
pub use memory::*;
pub use other::*;
pub use platform::*;
pub use sbi::*;
pub use time::*;
pub use trap::*;

/// full arch trait
pub trait FullVirtArch: ArchInt + ArchAsm + ArchSbi + ArchTrap + ArchTime + ArchInfo {
    // should impl all traits above
}

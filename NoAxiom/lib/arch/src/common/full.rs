use super::{ArchAsm, ArchBoot, ArchInt, ArchSbi, ArchTime, ArchTrap};

/// full arch trait
pub trait ArchFull: ArchInt + ArchAsm + ArchSbi + ArchTrap + ArchTime + ArchBoot {
    const ARCH_NAME: &'static str = "unknown";
}

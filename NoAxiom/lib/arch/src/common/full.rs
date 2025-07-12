use super::{ArchAsm, ArchBoot, ArchInfo, ArchInt, ArchTime, ArchTrap};

/// full arch trait
pub trait ArchFull: ArchInt + ArchAsm + ArchTrap + ArchTime + ArchBoot + ArchInfo {}

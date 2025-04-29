use super::{ArchAsm, ArchBoot, ArchInfo, ArchInt, ArchSbi, ArchTime, ArchTrap};

/// full arch trait
pub trait ArchFull:
    ArchInt + ArchAsm + ArchSbi + ArchTrap + ArchTime + ArchBoot + ArchInfo
{
}

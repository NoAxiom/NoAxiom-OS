use super::{ArchAsm, ArchBoot, ArchInfo, ArchInt, ArchMemory, ArchTime, ArchTrap};

/// full arch trait
pub trait ArchFull:
    ArchInt + ArchAsm + ArchTrap + ArchTime + ArchBoot + ArchInfo + ArchMemory
{
}

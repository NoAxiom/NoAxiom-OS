//! MapPermission for a MapArea

use bitflags::bitflags;

use super::pte::PTEFlags;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    /// Identical mapping, no translation
    Identical,

    /// Framed mapping, with page table translation
    Framed,

    /// Direct mapping, with simple translation
    Direct,
}

bitflags! {
    /// map permission contains kernel-associated pte flags,
    /// and don't contain hardware-associated pte flags
    #[derive(Clone, Copy, Debug)]
    pub struct MapPermission: u16 {
        /// readable
        const R = 1 << 1;
        /// writable
        const W = 1 << 2;
        /// executable
        const X = 1 << 3;
        /// user accesible
        const U = 1 << 4;
    }
}

impl MapPermission {
    pub fn readable(&self) -> bool {
        self.contains(MapPermission::R)
    }
    pub fn writable(&self) -> bool {
        self.contains(MapPermission::W)
    }
    pub fn executable(&self) -> bool {
        self.contains(MapPermission::X)
    }
    pub fn is_user(&self) -> bool {
        self.contains(MapPermission::U)
    }
    pub fn into_pte_flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits()).unwrap()
    }
    pub fn merge_from_elf_flags(mut self, flags: xmas_elf::program::Flags) -> Self {
        self.set(MapPermission::R, flags.is_read());
        self.set(MapPermission::W, flags.is_write());
        self.set(MapPermission::X, flags.is_execute());
        self
    }
}

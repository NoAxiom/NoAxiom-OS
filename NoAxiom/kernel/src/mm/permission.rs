//! MapPermission for a MapArea

use arch::MappingFlags;
use bitflags::bitflags;

#[allow(unused)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    /// Identical mapping, no translation, not used
    Identical,

    /// Framed mapping, with page table translation
    Framed,

    /// Direct mapping, with simple translation, only for kernel
    Direct,

    /// Shifted mapping, with page table translation
    /// isize is the shift offset (phys page num offset)
    Linear { ppn_offset: isize },
}

bitflags! {
    /// map permission contains kernel-associated pte flags,
    /// and don't contain hardware-associated pte flags
    #[derive(Clone, Copy, Debug)]
    pub struct MapPermission: usize {
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

#[allow(unused)]
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
    pub fn merge_from_elf_flags(mut self, flags: xmas_elf::program::Flags) -> Self {
        self.set(MapPermission::R, flags.is_read());
        self.set(MapPermission::W, flags.is_write());
        self.set(MapPermission::X, flags.is_execute());
        self
    }
}

impl Into<MappingFlags> for MapPermission {
    fn into(self) -> MappingFlags {
        let mut res = MappingFlags::empty();
        if self.contains(MapPermission::R) {
            res |= MappingFlags::R;
        }
        if self.contains(MapPermission::W) {
            res |= MappingFlags::W;
        }
        if self.contains(MapPermission::X) {
            res |= MappingFlags::X;
        }
        if self.contains(MapPermission::U) {
            res |= MappingFlags::U;
        }
        res
    }
}

//! Map Permission for

use bitflags::bitflags;
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
    Direct,
}

bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct MapPermission: u8 {
        // const V = 1 << 0;
        /// readable
        const R = 1 << 1;

        /// writable
        const W = 1 << 2;

        /// executable
        const X = 1 << 3;

        /// user accesible
        const U = 1 << 4;
        // const G = 1 << 5;
        // const A = 1 << 6;
        // const D = 1 << 7;
        // const COW = 1 << 8;
    }
}

impl MapPermission {
    pub fn readable(self) -> bool {
        self.contains(MapPermission::R)
    }
    pub fn writable(self) -> bool {
        self.contains(MapPermission::W)
    }
    pub fn executable(self) -> bool {
        self.contains(MapPermission::X)
    }
    pub fn is_user(self) -> bool {
        self.contains(MapPermission::U)
    }
}

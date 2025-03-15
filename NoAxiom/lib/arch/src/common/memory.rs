use bitflags::bitflags;

use crate::utils::macros::bit;

bitflags! {
    /// Mapping flags for page table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MappingFlags: u64 {
        /// Persent
        const P = bit!(0);
        /// User Accessable Flag
        const U = bit!(1);
        /// Readable Flag
        const R = bit!(2);
        /// Writeable Flag
        const W = bit!(3);
        /// Executeable Flag
        const X = bit!(4);
        /// Accessed Flag
        const A = bit!(5);
        /// Dirty Flag, indicating that the page was written
        const D = bit!(6);
        /// Global Flag
        const G = bit!(7);
        /// Device Flag, indicating that the page was used for device memory
        const Device = bit!(8);
        /// Cache Flag, indicating that the page will be cached
        const Cache = bit!(9);
        /// Copy on write flag
        const COW = bit!(10);

        /// Read | Write | Executeable Flags
        const RWX = Self::R.bits() | Self::W.bits() | Self::X.bits();
        /// User | Read | Write Flags
        const URW = Self::U.bits() | Self::R.bits() | Self::W.bits();
        /// User | Read | Executeable Flags
        const URX = Self::U.bits() | Self::R.bits() | Self::X.bits();
        /// User | Read | Write | Executeable Flags
        const URWX = Self::URW.bits() | Self::X.bits();
    }
}

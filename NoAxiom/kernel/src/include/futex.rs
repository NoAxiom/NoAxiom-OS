use bitflags::bitflags;
use strum::FromRepr;

#[repr(usize)]
#[derive(FromRepr)]
#[derive(Debug)]
pub enum FutexOps {
    FutexWait = 0,
    FutexWake = 1,
    FutexFd = 2,
    FutexRequeue = 3,
    FutexCmpRequeue = 4,
    FutexWakeOp = 5,
    FutexLockPi = 6,
    FutexUnlockPi = 7,
    FutexTrylockPi = 8,
    FutexWaitBitset = 9,
    FutexWakeBitset = 10,
    FutexWaitRequeuePi = 11,
    FutexCmpRequeuePi = 12,
    FutexLockPi2 = 13,
}

impl FutexOps {
    pub fn is_futex_wake(&self) -> bool {
        matches!(
            self,
            FutexOps::FutexWake | FutexOps::FutexWakeOp | FutexOps::FutexWakeBitset
        )
    }
}

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FutexFlags: usize {
        const FUTEX_PRIVATE = 128;
        const FUTEX_CLOCK_REALTIME = 256;
        const FUTEX_FLAG_MASK = Self::FUTEX_PRIVATE.bits() | Self::FUTEX_CLOCK_REALTIME.bits();
        const FUTEX_CMD_MASK = !Self::FUTEX_FLAG_MASK.bits();
    }
}

impl FutexFlags {
    pub fn is_private(&self) -> bool {
        self.contains(Self::FUTEX_PRIVATE)
    }
    pub fn is_clock_realtime(&self) -> bool {
        self.contains(Self::FUTEX_CLOCK_REALTIME)
    }
}

pub const FUTEX_BITSET_MATCH_ANY: u32 = u32::MAX;

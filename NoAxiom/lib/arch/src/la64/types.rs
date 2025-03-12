use super::LA64;
use crate::ArchType;

impl ArchType for LA64 {
    type Exception = usize;
    type Interrupt = usize;
    type Trap = usize;
    type TrapContext = usize;
}

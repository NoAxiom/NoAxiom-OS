use super::{context, RV64};
use crate::ArchType;

impl ArchType for RV64 {
    type Trap = riscv::register::scause::Trap;
    type Interrupt = riscv::register::scause::Interrupt;
    type Exception = riscv::register::scause::Exception;
    type TrapContext = context::TrapContext;
}

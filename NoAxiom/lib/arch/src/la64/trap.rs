use core::ops::{Index, IndexMut};

use super::LA64;
use crate::{ArchTrap, ArchTrapContext, TrapArgs};

/// Saved registers when a trap (interrupt or exception) occurs.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapContext {
    /// General Registers
    pub regs: [usize; 32],
    /// Pre-exception Mode information
    pub prmd: usize,
    /// Exception Return Address
    pub era: usize,
}

impl TrapContext {
    // 创建上下文信息
    #[inline]
    pub fn new() -> Self {
        Self {
            // bit 1:0 PLV
            // bit 2 PIE
            // bit 3 PWE
            prmd: (0b0111),
            ..Default::default()
        }
    }
}

impl TrapContext {
    pub fn syscall_ok(&mut self) {
        self.era += 4;
    }

    #[inline]
    pub fn args(&self) -> [usize; 6] {
        [
            self.regs[4],
            self.regs[5],
            self.regs[6],
            self.regs[7],
            self.regs[8],
            self.regs[9],
        ]
    }
}

impl Index<TrapArgs> for TrapContext {
    type Output = usize;

    fn index(&self, index: TrapArgs) -> &Self::Output {
        match index {
            TrapArgs::EPC => &self.era,
            TrapArgs::RA => &self.regs[1],
            TrapArgs::SP => &self.regs[3],
            TrapArgs::RES => &self.regs[4],
            TrapArgs::A0 => &self.regs[4],
            TrapArgs::A1 => &self.regs[5],
            TrapArgs::A2 => &self.regs[6],
            TrapArgs::A3 => &self.regs[7],
            TrapArgs::A4 => &self.regs[8],
            TrapArgs::A5 => &self.regs[9],
            TrapArgs::TLS => &self.regs[2],
            TrapArgs::SYSCALL => &self.regs[11],
        }
    }
}

impl IndexMut<TrapArgs> for TrapContext {
    fn index_mut(&mut self, index: TrapArgs) -> &mut Self::Output {
        match index {
            TrapArgs::EPC => &mut self.era,
            TrapArgs::RA => &mut self.regs[1],
            TrapArgs::SP => &mut self.regs[3],
            TrapArgs::RES => &mut self.regs[4],
            TrapArgs::A0 => &mut self.regs[4],
            TrapArgs::A1 => &mut self.regs[5],
            TrapArgs::A2 => &mut self.regs[6],
            TrapArgs::A3 => &mut self.regs[7],
            TrapArgs::A4 => &mut self.regs[8],
            TrapArgs::A5 => &mut self.regs[9],
            TrapArgs::TLS => &mut self.regs[2],
            TrapArgs::SYSCALL => &mut self.regs[11],
        }
    }
}

impl ArchTrapContext for TrapContext {
}

impl ArchTrap for LA64 {
    type TrapContext = TrapContext;
}

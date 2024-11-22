use core::arch::asm;

use riscv::register::sstatus::SPP;

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct Sstatus(pub usize);

impl Sstatus {
    pub fn read() -> Self {
        let val: usize;
        unsafe {
            asm!("csrr {},sstatus", out(reg)val);
        }
        Sstatus(val)
    }
    pub fn set_value(&mut self, val: usize) {
        self.0 = val;
    }
    pub fn set_spp(&mut self, spp: SPP) {
        self.0 = self.0 & !(1 << 8) | ((spp as usize) << 8);
    }
    pub fn spp(&self) -> SPP {
        let v = (self.0 >> 8) & 1;
        if v == 1 {
            SPP::Supervisor
        } else {
            SPP::User
        }
    }
    pub fn set_spie(&mut self) {
        self.0 |= 1 << 5;
    }
    pub fn sie(&self) -> bool {
        (self.0 & (1 << 1)) != 0
    }
    pub fn set_sie(&mut self, value: bool) {
        self.0 = self.0 & !(1 << 1) | ((value as usize) << 1);
    }
}

#[inline(always)]
pub fn get_sp() -> usize {
    let sp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

#[inline(always)]
pub fn get_tp() -> usize {
    let tp: usize;
    unsafe {
        core::arch::asm!("mv {}, sp", out(reg) tp);
    }
    tp
}

pub mod gpr_const {
    pub const ZERO: usize = 0;
    pub const RA: usize = 1;
    pub const SP: usize = 2;
    pub const GP: usize = 3;
    pub const TP: usize = 4;
    pub const T0: usize = 5;
    pub const T1: usize = 6;
    pub const T2: usize = 7;
    pub const S0: usize = 8;
    pub const FP: usize = 8;
    pub const S1: usize = 9;
    pub const A0: usize = 10;
    pub const A1: usize = 11;
    pub const A2: usize = 12;
    pub const A3: usize = 13;
    pub const A4: usize = 14;
    pub const A5: usize = 15;
    pub const A6: usize = 16;
    pub const A7: usize = 17;
    pub const S2: usize = 18;
    pub const S3: usize = 19;
    pub const S4: usize = 20;
    pub const S5: usize = 21;
    pub const S6: usize = 22;
    pub const S7: usize = 23;
    pub const S8: usize = 24;
    pub const S9: usize = 25;
    pub const S10: usize = 26;
    pub const S11: usize = 27;
    pub const T3: usize = 28;
    pub const T4: usize = 29;
    pub const T5: usize = 30;
    pub const T6: usize = 31;
}

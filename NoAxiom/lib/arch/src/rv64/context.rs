//! trap 上下文
//!
//! [`TrapContext`] 布局
//!
//! X0	               zero	     零寄存器，可做源寄存器(rs)或目标寄存器(rd)
//! X1	               ra	     链接寄存器，保存函数返回地址
//! X2	               sp	     栈指针寄存器，指向栈的地址
//! X3	               gp	     全局寄存器，用于链接器松弛优化
//! X4	               tp	     线程寄存器
//! X5 ~ X7 X28 ~ X31  t0 ~ t6   常用于在OS中保存指向进程控制块数据结构的指针
//! X8	               s0/fp	 临时寄存器/帧指针寄存器
//! X9	               s1        用于函数调用，被调用函数需保存数据
//! X10 ~ X17	       a0 ~ a7   用于函数调用，被调用函数需要保存的数据
//! X18 ~ X27	       s2 ~ s11  用于函数调用，传递参数和返回值

use core::{
    arch::asm,
    ops::{Index, IndexMut},
};

use riscv::register::sstatus::SPP;

use crate::{ArchTrapContext, TrapArgs};

pub mod reg_id {
    #![allow(unused)]
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

/// virtual sstatus register, it's not a real register
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Sstatus(pub usize);

#[allow(unused)]
impl Sstatus {
    pub fn read() -> Self {
        let val: usize;
        unsafe {
            asm!("csrr {},sstatus", out(reg)val);
        }
        Sstatus(val)
    }
    pub fn sum() -> usize {
        (Self::read().0 >> 18) & 1
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

/// Trap Context
/// save registers when trap occurs
/// we don't expect this to derive Clone
#[repr(C)]
#[repr(align(64))]
#[derive(Debug, Clone)]
pub struct TrapContext {
    /// [0~31]/[0~255]: user registers, saved by caller
    x: [usize; 32],

    /// [32]/[256~263]: cpu status
    sstatus: Sstatus,

    /// [33]/[264~271]: exception pc (va)
    sepc: usize,

    /// [34]/[272~279]: kernel stack top (va)
    kernel_sp: usize,

    /// [35]/[280~287]: kernel return address (va),
    /// returns to this addr when utrap happens,
    /// actually returns to async func
    kernel_ra: usize,

    /// [36~47]/[288~383]: kernel registers (s0 ~ s11), saved by callee
    kernel_reg: [usize; 12],

    /// [48]/[384~391]: reserved
    kernel_fp: usize,

    /// [49]/[392~399]: tp, aka hartid
    kernel_tp: usize,
}

impl Index<TrapArgs> for TrapContext {
    type Output = usize;

    fn index(&self, index: TrapArgs) -> &Self::Output {
        match index {
            TrapArgs::EPC => &self.sepc,
            TrapArgs::RA => &self.x[reg_id::RA],
            TrapArgs::SP => &self.x[reg_id::SP],
            TrapArgs::RES => &self.x[reg_id::A0],
            TrapArgs::A0 => &self.x[reg_id::A0],
            TrapArgs::A1 => &self.x[reg_id::A1],
            TrapArgs::A2 => &self.x[reg_id::A2],
            TrapArgs::A3 => &self.x[reg_id::A3],
            TrapArgs::A4 => &self.x[reg_id::A4],
            TrapArgs::A5 => &self.x[reg_id::A5],
            TrapArgs::TLS => &self.x[reg_id::TP],
            TrapArgs::SYSCALL => &self.x[reg_id::A7],
        }
    }
}

impl IndexMut<TrapArgs> for TrapContext {
    fn index_mut(&mut self, index: TrapArgs) -> &mut Self::Output {
        match index {
            TrapArgs::EPC => &mut self.sepc,
            TrapArgs::RA => &mut self.x[reg_id::RA],
            TrapArgs::SP => &mut self.x[reg_id::SP],
            TrapArgs::RES => &mut self.x[reg_id::A0],
            TrapArgs::A0 => &mut self.x[reg_id::A0],
            TrapArgs::A1 => &mut self.x[reg_id::A1],
            TrapArgs::A2 => &mut self.x[reg_id::A2],
            TrapArgs::A3 => &mut self.x[reg_id::A3],
            TrapArgs::A4 => &mut self.x[reg_id::A4],
            TrapArgs::A5 => &mut self.x[reg_id::A5],
            TrapArgs::TLS => &mut self.x[reg_id::TP],
            TrapArgs::SYSCALL => &mut self.x[reg_id::A7],
        }
    }
}

impl ArchTrapContext for TrapContext {
    fn app_init_cx(entry: usize, sp: usize) -> Self {
        let mut sstatus = Sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_sp: 0,
            kernel_ra: 0,
            kernel_reg: [0; 12],
            kernel_fp: 0,
            kernel_tp: 0,
        };
        cx.x[reg_id::SP] = sp;
        cx
    }
    fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize, envp: usize) {
        use TrapArgs::*;
        self.sepc = entry;
        self[SP] = sp;
        self[A0] = argc;
        self[A1] = argv;
        self[A2] = envp;
        let mut sstatus = Sstatus::read();
        sstatus.set_spp(SPP::User);
        self.sstatus = sstatus;
    }
}

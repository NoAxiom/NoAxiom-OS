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

#![allow(dead_code)]

use core::{
    arch::asm,
    ops::{Index, IndexMut},
};

use bit_field::BitField;
use riscv::register::sstatus::{self, FS, SPP};

use crate::{ArchTrapContext, ArchUserFloatContext, TrapArgs};

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
pub struct MySstatus(pub usize);

impl MySstatus {
    pub fn read() -> Self {
        let val: usize;
        unsafe {
            asm!("csrr {},sstatus", out(reg)val);
        }
        MySstatus(val)
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
    pub fn set_fs(&mut self, fs: FS) {
        let v: u8 = unsafe { core::mem::transmute(fs) };
        self.0.set_bits(13..15, v as usize);
    }
    pub fn fs(&self) -> FS {
        match self.0.get_bits(13..15) {
            0 => FS::Off,
            1 => FS::Initial,
            2 => FS::Clean,
            3 => FS::Dirty,
            _ => unreachable!(),
        }
    }
}

/// Trap Context
/// save registers when trap occurs
/// we don't expect this to derive Clone
#[repr(C)]
#[repr(align(64))]
#[derive(Default, Clone)]
pub struct TrapContext {
    /// [0~31]/[0~255]: user registers, saved by caller
    x: [usize; 32],

    /// [32]/[256~263]: cpu status
    sstatus: MySstatus,

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

    /// freg
    freg: UserFloatContext,
}

impl core::fmt::Debug for TrapContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapContext")
            .field("r0  / r0", &self.x[0])
            .field("r1  / ra", &self.x[1])
            .field("r2  / sp", &self.x[2])
            .field("r3  / gp", &self.x[3])
            .field("r4  / tp", &self.x[4])
            .field("r5  / t0", &self.x[5])
            .field("r6  / t1", &self.x[6])
            .field("r7  / t2", &self.x[7])
            .field("r8  / s0", &self.x[8])
            .field("r9  / s1", &self.x[9])
            .field("r10 / a0", &self.x[10])
            .field("r11 / a1", &self.x[11])
            .field("r12 / a2", &self.x[12])
            .field("r13 / a3", &self.x[13])
            .field("r14 / a4", &self.x[14])
            .field("r15 / a5", &self.x[15])
            .field("r16 / a6", &self.x[16])
            .field("r17 / a7", &self.x[17])
            .field("r18 / s2", &self.x[18])
            .field("r19 / s3", &self.x[19])
            .field("r20 / s4", &self.x[20])
            .field("r21 / s5", &self.x[21])
            .field("r22 / s6", &self.x[22])
            .field("r23 / s7", &self.x[23])
            .field("r24 / s8", &self.x[24])
            .field("r25 / s9", &self.x[25])
            .field("r26 /s10", &self.x[26])
            .field("r27 /s11", &self.x[27])
            .field("r28 / t3", &self.x[28])
            .field("r29 / t4", &self.x[29])
            .field("r30 / t5", &self.x[30])
            .field("r31 / t6", &self.x[31])
            .field("sstatus", &self.sstatus)
            .field("sepc", &self.sepc)
            .field("kernel_sp", &self.kernel_sp)
            .finish()?;
        if self.freg.user_fx.iter().any(|x| *x != 0f64) {
            write!(f, "\nFloat Register: {:?}", self.freg)?;
        } else {
            write!(f, "\nFloat Register: Empty")?;
        }
        core::fmt::Result::Ok(())
    }
}

impl TrapContext {
    #[inline(always)]
    pub(crate) fn sstatus(&self) -> MySstatus {
        self.sstatus
    }
    pub fn check(&self) {
        assert!(self.sstatus.spp() == SPP::User);
    }
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
    type FloatContext = UserFloatContext;
    fn freg_mut(&mut self) -> &mut Self::FloatContext {
        &mut self.freg
    }
    fn app_init_cx(entry: usize, sp: usize) -> Self {
        let mut sstatus = MySstatus::read();
        sstatus.set_spp(SPP::User);
        sstatus.set_spie();
        sstatus.set_sie(false);
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,
            kernel_sp: 0,
            kernel_ra: 0,
            kernel_reg: [0; 12],
            kernel_fp: 0,
            kernel_tp: 0,
            freg: UserFloatContext::new(),
        };
        cx.x[reg_id::SP] = sp;
        cx
    }
    // fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize,
    // envp: usize) {     use TrapArgs::*;
    //     self.sepc = entry;
    //     self[SP] = sp;
    //     self[A0] = argc;
    //     self[A1] = argv;
    //     self[A2] = envp;
    //     let sstatus = MySstatus::read();
    //     self.freg = UserFloatContext::new();
    //     // sstatus.set_spp(SPP::User);
    //     self.sstatus = sstatus;
    // }
    fn gprs(&self) -> &[usize; 32] {
        &self.x
    }
    fn gprs_mut(&mut self) -> &mut [usize; 32] {
        &mut self.x
    }
}

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct UserFloatContext {
    pub user_fx: [f64; 32], // 50 - 81
    pub fcsr: u32,          // 32bit
    pub need_save: u8,
    pub need_restore: u8,
}

pub fn freg_init() {
    unsafe { sstatus::set_fs(FS::Initial) };
}

impl ArchUserFloatContext for UserFloatContext {
    fn freg_init() {
        freg_init();
    }

    fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }

    fn yield_task(&mut self) {
        self.save();
        self.need_restore = 1;
    }

    /// Save reg -> mem
    fn save(&mut self) {
        if self.need_save == 0 {
            return;
        }
        self.need_save = 0;
        unsafe {
            let mut _t: usize = 1; // alloc a register but not zero.
            asm!("
                fsd  f0,  0*8({0})
                fsd  f1,  1*8({0})
                fsd  f2,  2*8({0})
                fsd  f3,  3*8({0})
                fsd  f4,  4*8({0})
                fsd  f5,  5*8({0})
                fsd  f6,  6*8({0})
                fsd  f7,  7*8({0})
                fsd  f8,  8*8({0})
                fsd  f9,  9*8({0})
                fsd f10, 10*8({0})
                fsd f11, 11*8({0})
                fsd f12, 12*8({0})
                fsd f13, 13*8({0})
                fsd f14, 14*8({0})
                fsd f15, 15*8({0})
                fsd f16, 16*8({0})
                fsd f17, 17*8({0})
                fsd f18, 18*8({0})
                fsd f19, 19*8({0})
                fsd f20, 20*8({0})
                fsd f21, 21*8({0})
                fsd f22, 22*8({0})
                fsd f23, 23*8({0})
                fsd f24, 24*8({0})
                fsd f25, 25*8({0})
                fsd f26, 26*8({0})
                fsd f27, 27*8({0})
                fsd f28, 28*8({0})
                fsd f29, 29*8({0})
                fsd f30, 30*8({0})
                fsd f31, 31*8({0})
                csrr {1}, fcsr
                sw  {1}, 32*8({0})
            ", in(reg) self, inout(reg) _t
            );
        };
    }

    /// Restore mem -> reg
    fn restore(&mut self) {
        if self.need_restore == 0 {
            return;
        }
        self.need_restore = 0;
        unsafe {
            asm!("
                fld  f0,  0*8({0})
                fld  f1,  1*8({0})
                fld  f2,  2*8({0})
                fld  f3,  3*8({0})
                fld  f4,  4*8({0})
                fld  f5,  5*8({0})
                fld  f6,  6*8({0})
                fld  f7,  7*8({0})
                fld  f8,  8*8({0})
                fld  f9,  9*8({0})
                fld f10, 10*8({0})
                fld f11, 11*8({0})
                fld f12, 12*8({0})
                fld f13, 13*8({0})
                fld f14, 14*8({0})
                fld f15, 15*8({0})
                fld f16, 16*8({0})
                fld f17, 17*8({0})
                fld f18, 18*8({0})
                fld f19, 19*8({0})
                fld f20, 20*8({0})
                fld f21, 21*8({0})
                fld f22, 22*8({0})
                fld f23, 23*8({0})
                fld f24, 24*8({0})
                fld f25, 25*8({0})
                fld f26, 26*8({0})
                fld f27, 27*8({0})
                fld f28, 28*8({0})
                fld f29, 29*8({0})
                fld f30, 30*8({0})
                fld f31, 31*8({0})
                lw  {0}, 32*8({0})
                csrw fcsr, {0}
            ", in(reg) self
            );
        }
    }
}

impl UserFloatContext {
    pub(crate) fn mark_save_if_needed(&mut self, fs: FS) {
        self.need_save |= (fs == FS::Dirty) as u8;
    }
}

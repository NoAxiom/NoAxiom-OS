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

use riscv::register::sstatus::{self, FS, SPP};

use crate::{
    rv64::registers::{gprs, MySstatus},
    ArchTrapContext, ArchUserFloatContext, TrapArgs,
};

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
            TrapArgs::RA => &self.x[gprs::RA],
            TrapArgs::SP => &self.x[gprs::SP],
            TrapArgs::RES => &self.x[gprs::A0],
            TrapArgs::A0 => &self.x[gprs::A0],
            TrapArgs::A1 => &self.x[gprs::A1],
            TrapArgs::A2 => &self.x[gprs::A2],
            TrapArgs::A3 => &self.x[gprs::A3],
            TrapArgs::A4 => &self.x[gprs::A4],
            TrapArgs::A5 => &self.x[gprs::A5],
            TrapArgs::TLS => &self.x[gprs::TP],
            TrapArgs::SYSCALL => &self.x[gprs::A7],
        }
    }
}

impl IndexMut<TrapArgs> for TrapContext {
    fn index_mut(&mut self, index: TrapArgs) -> &mut Self::Output {
        match index {
            TrapArgs::EPC => &mut self.sepc,
            TrapArgs::RA => &mut self.x[gprs::RA],
            TrapArgs::SP => &mut self.x[gprs::SP],
            TrapArgs::RES => &mut self.x[gprs::A0],
            TrapArgs::A0 => &mut self.x[gprs::A0],
            TrapArgs::A1 => &mut self.x[gprs::A1],
            TrapArgs::A2 => &mut self.x[gprs::A2],
            TrapArgs::A3 => &mut self.x[gprs::A3],
            TrapArgs::A4 => &mut self.x[gprs::A4],
            TrapArgs::A5 => &mut self.x[gprs::A5],
            TrapArgs::TLS => &mut self.x[gprs::TP],
            TrapArgs::SYSCALL => &mut self.x[gprs::A7],
        }
    }
}

/// machine context
/// restores CPU's context
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SigContext {
    /// user general regs
    user_x: [usize; 32],
    /// float reg state, currently unused
    fpstate: [usize; 66],
}

impl SigContext {
    pub fn from_cx(value: &TrapContext) -> Self {
        let mut res = Self {
            // sepc:
            user_x: value.gprs().clone(),
            fpstate: [0; 66],
        };
        res.user_x[0] = value[TrapArgs::EPC];
        res
    }
    pub fn epc(&self) -> usize {
        self.user_x[0]
    }
    pub fn gprs(&self) -> [usize; 32] {
        self.user_x
    }
}

impl ArchTrapContext for TrapContext {
    type FloatContext = UserFloatContext;
    type SigContext = SigContext;
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
        cx.x[gprs::SP] = sp;
        cx
    }
    fn as_sig_cx(&self) -> Self::SigContext {
        SigContext::from_cx(self)
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

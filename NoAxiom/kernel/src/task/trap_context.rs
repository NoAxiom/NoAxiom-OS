//! trap 上下文
//!
//! [`TrapContext`] 布局
//!
//! ```text
//! +--------------+
//! |   通用寄存器   |
//! +--------------+ <-- offset: 256
//! |   sstatus    |
//! +--------------+ <-- offset: 264
//! |    sepc      |
//! +--------------+ <-- offset: 272
//! |  kernel_satp |
//! +--------------+ <-- offset: 280
//! |  kernel_sp   |
//! +--------------+ <-- offset: 288
//! | trap_handler |
//! +--------------+
//! ```
//!
//! X0	             zero	    零寄存器，可做源寄存器(rs)或目标寄存器(rd)
//! X1	             ra	        链接寄存器，保存函数返回地址
//! X2	             sp	        栈指针寄存器，指向栈的地址
//! X3	             gp	        全局寄存器，用于链接器松弛优化
//! X4	             tp	        线程寄存器
//! X5 ~ X7 X28 ~ X31 t0 ~ t6    常用于在OS中保存指向进程控制块数据结构的指针
//! X8	             s0/fp	    临时寄存器/帧指针寄存器
//! X9	             s1         用于函数调用，被调用函数需保存数据
//! X10 ~ X17	     a0 ~ a7    用于函数调用，被调用函数需要保存的数据
//! X18 ~ X27	     s2 ~ s11   用于函数调用，传递参数和返回值

use riscv::register::sstatus::{self, Sstatus, SPP};

const ZERO: usize = 0;
const RA: usize = 1;
const SP: usize = 2;
const GP: usize = 3;
const TP: usize = 4;
const T0: usize = 5;
const T1: usize = 6;
const T2: usize = 7;
const S0: usize = 8;
const FP: usize = 8;
const S1: usize = 9;
const A0: usize = 10;
const A1: usize = 11;
const A2: usize = 12;
const A3: usize = 13;
const A4: usize = 14;
const A5: usize = 15;
const A6: usize = 16;
const A7: usize = 17;
const S2: usize = 18;
const S3: usize = 19;
const S4: usize = 20;
const S5: usize = 21;
const S6: usize = 22;
const S7: usize = 23;
const S8: usize = 24;
const S9: usize = 25;
const S10: usize = 26;
const S11: usize = 27;
const T3: usize = 28;
const T4: usize = 29;
const T5: usize = 30;
const T6: usize = 31;

/// # Trap Context
/// save registers when trap occurs
pub struct TrapContext {
    /// 0: 32 general registers
    pub regs: [usize; 32],

    /// 32: cpu status
    pub sstatus: Sstatus,

    /// 33: exception pc
    pub sepc: usize,
    // 34: process kernel stack top (virtual address)
    // pub kernel_sp: usize,

    // 35
    // pub kernel_ra: usize,

    // 36 - 47
    // pub kernel_s: [usize; 12],

    // 48
    // pub kernel_fp: usize,

    // 49
    // pub cpu_id: usize,

    // 50: Floating point registers
    // pub freg: UserFloatContext,
}

impl TrapContext {
    pub fn app_init_cx(entry: usize, sp: usize) -> Self {
        let sstatus = sstatus::read();
        unsafe { sstatus::set_spp(SPP::User) };
        let mut cx = Self {
            regs: [0; 32],
            sstatus,
            sepc: entry,
            // kernel_sp: 0,
            // kernel_ra: 0,
            // kernel_s: [0; 12],
            // kernel_fp: 0,
            // todo: hart_id for multi-core
            // cpu_id: 0,
            // freg: UserFloatContext::new(),
        };
        cx.regs[SP] = sp;
        cx
    }
}

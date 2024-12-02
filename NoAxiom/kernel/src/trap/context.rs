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
//! X5 ~ X7 X28 ~ X31 t0 ~ t6   常用于在OS中保存指向进程控制块数据结构的指针
//! X8	             s0/fp	    临时寄存器/帧指针寄存器
//! X9	             s1         用于函数调用，被调用函数需保存数据
//! X10 ~ X17	     a0 ~ a7    用于函数调用，被调用函数需要保存的数据
//! X18 ~ X27	     s2 ~ s11   用于函数调用，传递参数和返回值

use riscv::register::sstatus::SPP;

use crate::{arch::regs::Sstatus, constant::register::*};

/// Trap Context
/// save registers when trap occurs
/// we don't expect this to derive Clone
#[repr(C)]
pub struct TrapContext {
    /// [0~31]/[0~255]: general registers
    pub regs: [usize; 32],

    /// [32]/[256~263]: cpu status
    pub sstatus: Sstatus,

    /// [33]/[264~271]: exception pc (va)
    pub sepc: usize,

    /// [34]/[272~279]: kernel stack top (va)
    pub kernel_sp: usize,

    /// [35]/[280~287]: kernel return address (va),
    /// returns to this addr when utrap happens,
    /// actually returns to async func
    pub kernel_ra: usize,
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
        info!("[app_init_cx] entry: {:#x}, sp: {:#x}", entry, sp);
        let mut sstatus = Sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            regs: [0; 32],
            sstatus,
            sepc: entry,
            kernel_sp: 0,
            kernel_ra: 0,
            // kernel_s: [0; 12],
            // kernel_fp: 0,
            // TODO: hart_id for multi-core
            // cpu_id: 0,
            // freg: UserFloatContext::new(),
        };
        cx.regs[SP] = sp;
        cx
    }
}

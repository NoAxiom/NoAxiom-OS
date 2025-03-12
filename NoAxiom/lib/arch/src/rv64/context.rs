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

use core::arch::asm;

use riscv::register::sstatus::SPP;

use super::register::reg_id;

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
    pub user_reg: [usize; 32],

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

    /// [36~47]/[288~383]: kernel registers (s0 ~ s11), saved by callee
    pub kernel_reg: [usize; 12],

    /// [48]/[384~391]: reserved
    pub kernel_fp: usize,

    /// [49]/[392~399]: tp, aka hartid
    pub kernel_tp: usize,
}

impl TrapContext {
    pub fn app_init_cx(entry: usize, sp: usize) -> Self {
        let mut sstatus = Sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Self {
            user_reg: [0; 32],
            sstatus,
            sepc: entry,
            kernel_sp: 0,
            kernel_ra: 0,
            kernel_reg: [0; 12],
            kernel_fp: 0,
            kernel_tp: 0,
        };
        cx.user_reg[reg_id::SP] = sp;
        cx
    }
    pub fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize, envp: usize) {
        self.sepc = entry;
        self.set_sp(sp);
        self.set_reg(reg_id::A0, argc);
        self.set_reg(reg_id::A1, argv);
        self.set_reg(reg_id::A2, envp);
        let mut sstatus = Sstatus::read();
        sstatus.set_spp(SPP::User);
        self.sstatus = sstatus;
    }
    #[inline(always)]
    pub fn set_sp(&mut self, sp: usize) {
        self.user_reg[reg_id::SP] = sp;
    }
    #[inline(always)]
    pub fn set_reg(&mut self, id: usize, value: usize) {
        self.user_reg[id] = value;
    }
    #[inline(always)]
    pub fn set_result(&mut self, value: usize) {
        self.user_reg[reg_id::A0] = value;
    }
    #[inline(always)]
    pub fn set_ra(&mut self, ra: usize) {
        self.user_reg[reg_id::RA] = ra;
    }
    #[inline(always)]
    pub fn result_value(&self) -> usize {
        self.user_reg[reg_id::A0]
    }
    #[inline(always)]
    pub fn get_syscall_id(&self) -> usize {
        self.user_reg[reg_id::A7]
    }
    #[inline(always)]
    pub fn get_syscall_args(&self) -> [usize; 6] {
        [
            self.user_reg[reg_id::A0],
            self.user_reg[reg_id::A1],
            self.user_reg[reg_id::A2],
            self.user_reg[reg_id::A3],
            self.user_reg[reg_id::A4],
            self.user_reg[reg_id::A5],
        ]
    }
}

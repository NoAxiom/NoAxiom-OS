use core::ops::{Index, IndexMut};

use crate::{ArchTrapContext, ArchUserFloatContext, TrapArgs};

/// Saved registers when a trap (interrupt or exception) occurs.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TrapContext {
    /// [0~31]/[0~255]: General Registers
    pub x: [usize; 32],
    /// [32]/[256~263]: Pre-exception Mode information
    pub prmd: usize,
    /// [33]/[264~271]: Exception Return Address
    pub era: usize,

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

impl TrapContext {
    // new trap context with priv level initialized
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
    pub fn set_prmd(&mut self) {
        self.prmd |= 0b0111;
    }
}

impl Index<TrapArgs> for TrapContext {
    type Output = usize;
    fn index(&self, index: TrapArgs) -> &Self::Output {
        match index {
            TrapArgs::EPC => &self.era,
            TrapArgs::RA => &self.x[1],
            TrapArgs::SP => &self.x[3],
            TrapArgs::RES => &self.x[4],
            TrapArgs::A0 => &self.x[4],
            TrapArgs::A1 => &self.x[5],
            TrapArgs::A2 => &self.x[6],
            TrapArgs::A3 => &self.x[7],
            TrapArgs::A4 => &self.x[8],
            TrapArgs::A5 => &self.x[9],
            TrapArgs::TLS => &self.x[2],
            TrapArgs::SYSCALL => &self.x[11],
        }
    }
}

impl IndexMut<TrapArgs> for TrapContext {
    fn index_mut(&mut self, index: TrapArgs) -> &mut Self::Output {
        match index {
            TrapArgs::EPC => &mut self.era,
            TrapArgs::RA => &mut self.x[1],
            TrapArgs::SP => &mut self.x[3],
            TrapArgs::RES => &mut self.x[4],
            TrapArgs::A0 => &mut self.x[4],
            TrapArgs::A1 => &mut self.x[5],
            TrapArgs::A2 => &mut self.x[6],
            TrapArgs::A3 => &mut self.x[7],
            TrapArgs::A4 => &mut self.x[8],
            TrapArgs::A5 => &mut self.x[9],
            TrapArgs::TLS => &mut self.x[2],
            TrapArgs::SYSCALL => &mut self.x[11],
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct UserFloatContext {
    pub user_fx: [f64; 32], // 50 - 81
    pub fcsr: u32,          // 32bit
    pub need_save: u8,
    pub need_restore: u8,
    pub signal_dirty: u8,
}

impl ArchUserFloatContext for UserFloatContext {
    fn save(&mut self) {
        todo!()
    }
    fn restore(&mut self) {
        todo!()
    }
    fn mark_save_if_needed(&mut self) {
        todo!()
    }
    fn new() -> Self {
        todo!()
    }
    fn yield_task(&mut self) {
        todo!()
    }
}

impl Default for UserFloatContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchTrapContext for TrapContext {
    type FloatContext = UserFloatContext;

    fn freg_mut(&mut self) -> &mut Self::FloatContext {
        &mut self.freg
    }

    fn gprs(&self) -> &[usize; 32] {
        &self.x
    }

    fn gprs_mut(&mut self) -> &mut [usize; 32] {
        &mut self.x
    }

    fn app_init_cx(entry: usize, sp: usize) -> Self {
        // Self::new contains priv level settings
        let mut cx = Self::new();
        use TrapArgs::*;
        cx[RA] = entry;
        cx[SP] = sp;
        cx
    }

    fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize, envp: usize) {
        use TrapArgs::*;
        self[RA] = entry;
        self[SP] = sp;
        self[A0] = argc;
        self[A1] = argv;
        self[A2] = envp;
        self.set_prmd();
    }
}

use core::ops::{Index, IndexMut};

use loongArch64::register::{euen, prmd};

use crate::{ArchTrapContext, ArchUserFloatContext, TrapArgs};

/// Saved registers when a trap (interrupt or exception) occurs.
#[repr(C)]
#[repr(align(64))]
#[derive(Default, Clone)]
pub struct TrapContext {
    /// [0~31]/[0~255]: General Registers
    pub(crate) x: [usize; 32],
    /// [32]/[256~263]: Pre-exception Mode information
    pub(crate) prmd: usize,
    /// [33]/[264~271]: Exception Return Address
    pub(crate) era: usize,

    /// [34]/[272~279]: kernel stack top (va)
    kernel_sp: usize,

    /// [35]/[280~287]: kernel return address (va),
    /// returns to this addr when utrap happens,
    /// actually returns to async func
    kernel_ra: usize,

    /// [36~47]/[288~383]: kernel registers (s0 ~ s11), saved by callee
    /// fixme: actually we should only save s0 ~ s9
    kernel_reg: [usize; 12],

    /// [48]/[384~391]: reserved
    kernel_fp: usize,

    /// [49]/[392~399]: tp
    kernel_tp: usize,

    /// freg
    freg: UserFloatContext,
}

impl TrapContext {
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
        self.prmd = prmd::read().raw();
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

impl core::fmt::Debug for TrapContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TrapContext")
            .field("r0  / r0", &self.x[0])
            .field("r1  / ra", &self.x[1])
            .field("r2  / tp", &self.x[2])
            .field("r3  / sp", &self.x[3])
            .field("r4  / a0", &self.x[4])
            .field("r5  / a1", &self.x[5])
            .field("r6  / a2", &self.x[6])
            .field("r7  / a3", &self.x[7])
            .field("r8  / a4", &self.x[8])
            .field("r9  / a5", &self.x[9])
            .field("r10 / a6", &self.x[10])
            .field("r11 / a7", &self.x[11])
            .field("r12 / t0", &self.x[12])
            .field("r13 / t1", &self.x[13])
            .field("r14 / t2", &self.x[14])
            .field("r15 / t3", &self.x[15])
            .field("r16 / t4", &self.x[16])
            .field("r17 / t5", &self.x[17])
            .field("r18 / t6", &self.x[18])
            .field("r19 / t7", &self.x[19])
            .field("r20 / t8", &self.x[20])
            .field("r21 / ??", &self.x[21])
            .field("r22 / fp", &self.x[22])
            .field("r23 / s0", &self.x[23])
            .field("r24 / s1", &self.x[24])
            .field("r25 / s2", &self.x[25])
            .field("r26 / s3", &self.x[26])
            .field("r27 / s4", &self.x[27])
            .field("r28 / s5", &self.x[28])
            .field("r29 / s6", &self.x[29])
            .field("r30 / s7", &self.x[30])
            .field("r31 / s8", &self.x[31])
            .field("prmd", &self.prmd)
            .field("era", &self.era)
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct UserFloatContext {
    pub user_fx: [f64; 32],
    pub fcsr: u32,
    pub fcc: u8, // should be u8?
    pub need_save: u8,
    pub need_restore: u8,
    pub signal_dirty: u8,
}

pub fn freg_init() {
    euen::set_fpe(true);
}

extern "C" {
    fn __save_freg(cx: *mut UserFloatContext);
    fn __load_freg(cx: *mut UserFloatContext);
}

impl ArchUserFloatContext for UserFloatContext {
    fn freg_init() {
        freg_init();
    }
    fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }
    fn save(&mut self) {
        if self.need_save == 0 {
            return;
        }
        self.need_save = 0;
        unsafe { __save_freg(self) };
    }
    fn restore(&mut self) {
        if self.need_restore == 0 {
            return;
        }
        self.need_restore = 0;
        unsafe { __load_freg(self) };
    }
    fn yield_task(&mut self) {
        self.save();
        self.need_restore = 1;
    }
}

impl Default for UserFloatContext {
    fn default() -> Self {
        Self::new()
    }
}

impl UserFloatContext {
    pub(crate) fn mark_save_if_needed(&mut self) {
        self.need_save = 1;
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
        let mut cx = Self::new();
        use TrapArgs::*;
        cx.set_prmd();
        cx[EPC] = entry;
        cx[SP] = sp;
        // fixme: should disable interrupt
        cx
    }

    // fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize,
    // envp: usize) {     use TrapArgs::*;
    //     self.set_prmd();
    //     self[EPC] = entry;
    //     self[SP] = sp;
    //     self[A0] = argc;
    //     self[A1] = argv;
    //     self[A2] = envp;
    //     self.freg = UserFloatContext::new();
    // }
}

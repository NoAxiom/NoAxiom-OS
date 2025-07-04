use arch::{ArchTrapContext, TrapArgs, TrapContext};

use super::sig_set::SigMask;

/// signal alternate stack
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SigAltStack {
    /// base stack pointer of stack
    pub ss_sp: usize,
    /// stack flags
    pub ss_flags: i32,
    /// stack size (bytes)
    pub ss_size: usize,
}

impl Default for SigAltStack {
    fn default() -> Self {
        Self::new_bare()
    }
}

impl SigAltStack {
    pub fn new_bare() -> Self {
        SigAltStack {
            ss_sp: 0usize,
            ss_flags: 0,
            ss_size: 0,
        }
    }
    #[inline(always)]
    pub fn stack_top(&self) -> usize {
        self.ss_sp + self.ss_size
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct UContext {
    pub uc_flags: usize,
    /// when current context returns, will restore the context at *uc_link
    pub uc_link: usize,
    /// current context's stack info, containing base sp, flags, size
    pub uc_stack: SigAltStack,
    /// when the ucontext is activated, will block sigs by this sigmask
    pub uc_sigmask: SigMask,
    /// padding
    pub __unused: [usize; 1024 / 8 - core::mem::size_of::<SigMask>()],
    /// machine context
    pub uc_mcontext: MContext,
}

/// machine context
/// restores CPU's context
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MContext {
    /// user general regs
    user_x: [usize; 32],
    /// float reg state, currently unused
    fpstate: [usize; 66],
}

impl MContext {
    pub fn from_cx(value: &TrapContext) -> Self {
        let mut res = Self {
            user_x: value.gprs().clone(),
            fpstate: [0; 66],
        };
        // fixme: is this correct in LA64?
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

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
    // don't know why, struct need to be exact the same with musl libc
    pub uc_sig: [usize; 16],
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
    pub fn new_bare() -> Self {
        Self {
            user_x: [0; 32],
            fpstate: [0; 66],
        }
    }
    pub fn from_cx(value: &TrapContext) -> Self {
        let mut res = Self {
            user_x: *value.gprs(),
            fpstate: [0; 66],
        };
        res.user_x[0] = value[TrapArgs::EPC];
        res
    }
}

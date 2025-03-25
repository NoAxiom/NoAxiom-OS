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

impl SigAltStack {
    fn new_bare() -> Self {
        SigAltStack {
            ss_sp: 0usize.into(),
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
    pub user_x: [usize; 32],
    /// float reg state, currently unused
    pub fpstate: [usize; 66],
}

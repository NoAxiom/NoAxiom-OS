use arch::{ArchTrapContext, TrapContext};

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

pub type SigContext = <TrapContext as ArchTrapContext>::SigContext;
const RAW_SIGMASK_BITS: usize = 1024;
pub const UCONTEXT_UNUSED_SIZE: usize = RAW_SIGMASK_BITS / 8 - core::mem::size_of::<SigMask>();

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
    /// There's some padding here to allow sigset_t to be expanded in the
    /// future.  Though this is unlikely, other architectures put uc_sigmask
    /// at the end of this structure and explicitly state it can be
    /// expanded, so we didn't want to box ourselves in here.
    pub __unused: [u8; UCONTEXT_UNUSED_SIZE],
    /// machine context
    pub uc_mcontext: SigContext,
}

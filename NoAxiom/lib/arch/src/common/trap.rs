use core::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy)]
pub enum TrapType {
    Breakpoint,
    SysCall,
    Timer,
    Unknown,
    SupervisorExternal,
    SupervisorSoft,
    StorePageFault(usize),
    LoadPageFault(usize),
    InstructionPageFault(usize),
    IllegalInstruction(usize),
    None,
}

/// Trap Frame Arg Type
///
/// Using this by Index and IndexMut trait bound on TrapFrame
#[derive(Debug, Clone, Copy)]
pub enum TrapArgs {
    EPC,
    RA,
    SP,
    RES,
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    TLS,
    SYSCALL,
}

/// trap related arch trait
pub trait ArchTrap {
    type TrapContext: ArchTrapContext;
    fn trap_init();
    fn trap_restore(_cx: &mut <Self as ArchTrap>::TrapContext);
    fn set_kernel_trap_entry();
    fn set_user_trap_entry();
    fn read_epc() -> usize;
    fn read_trap_type(cx: Option<&mut <Self as ArchTrap>::TrapContext>) -> TrapType;
}

pub trait ArchTrapContext:
    Index<TrapArgs, Output = usize> + IndexMut<TrapArgs, Output = usize>
{
    type FloatContext: ArchUserFloatContext;
    fn app_init_cx(entry: usize, sp: usize) -> Self;
    fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize, envp: usize);
    fn freg_mut(&mut self) -> &mut Self::FloatContext;
    fn gprs(&self) -> &[usize; 32];
    fn gprs_mut(&mut self) -> &mut [usize; 32];
    fn get_syscall_id(&self) -> usize {
        self[TrapArgs::SYSCALL]
    }
    fn get_syscall_args(&self) -> [usize; 6] {
        [
            self[TrapArgs::A0],
            self[TrapArgs::A1],
            self[TrapArgs::A2],
            self[TrapArgs::A3],
            self[TrapArgs::A4],
            self[TrapArgs::A5],
        ]
    }
}

pub trait ArchUserFloatContext {
    fn new() -> Self;
    fn save(&mut self);
    fn restore(&mut self);
    fn mark_save_if_needed(&mut self);
    fn yield_task(&mut self);
    fn encounter_signal(&mut self) {
        self.save();
    }
}

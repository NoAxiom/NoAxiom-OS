use arch::{Arch, ArchTrap, TrapArgs, TrapContext, TrapType};

use super::Task;
use crate::syscall::utils::clear_current_syscall;

pub struct TaskTrapContext {
    pub cx: TrapContext,
    pub res_tmp: usize, // for restart
    pub int_en: bool,
}

impl TaskTrapContext {
    pub fn new(cx: TrapContext, int_en: bool) -> Self {
        Self {
            cx,
            res_tmp: 0,
            int_en,
        }
    }
    pub fn cx(&self) -> &TrapContext {
        &self.cx
    }
    pub fn cx_mut(&mut self) -> &mut TrapContext {
        &mut self.cx
    }
}

impl Task {
    pub fn trap_restore(&self) -> TrapType {
        let task = self;
        clear_current_syscall();
        task.time_stat_mut().record_trap_in();
        let cx = task.trap_context_mut();
        Arch::trap_restore(cx); // restore context and return to user mode
        let trap_type = Arch::read_trap_type(cx);
        task.time_stat_mut().record_trap_out();
        trap_type
    }
    pub fn record_current_result_reg(&self) {
        self.tcb_mut().cx.res_tmp = self.trap_context_mut()[TrapArgs::RES];
    }
}

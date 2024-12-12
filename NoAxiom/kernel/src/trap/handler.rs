//! trap handler

use alloc::sync::Arc;

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, stval,
};

use super::trap::set_kernel_trap_entry;
use crate::{
    constant::register::A0, cpu::get_hartid, sched::utils::yield_now, syscall::syscall, task::Task,
};

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    let scause = scause::read();
    let stval = stval::read();
    let sepc = sepc::read();
    panic!(
        "kernel trap!!! trap {:?} is unsupported, stval = {:#x}, error pc = {:#x}",
        scause.cause(),
        stval,
        sepc,
    );
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    trace!("[trap_handler] call trap handler");
    set_kernel_trap_entry();
    let mut cx = task.trap_context_mut();
    let scause = scause::read();
    let stval = stval::read();
    trace!("[trap_handler] handle begin, scause: {:?}, stval: {:#x}", scause.cause(), stval);
    match scause.cause() {
        // syscall
        Trap::Exception(exception) => match exception {
            Exception::UserEnvCall => {
                cx.sepc += 4;
                trace!("[syscall] doing syscall");
                let result = syscall(task, cx).await;
                trace!("[syscall] done! result {:#x}", result);
                cx = task.trap_context_mut();
                cx.user_reg[A0] = result as usize;
            }
            _ => panic!(
                "hart: {}, exception {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
                get_hartid(),
                scause.cause(),
                stval,
                cx.sepc
            ),
        },
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorTimer => {
                task.inc_prio();
                debug!(
                    "[SupervisorTimer] hart: {}, tid: {}",
                    get_hartid(),
                    task.tid(),
                );
                yield_now().await;
            }
            _ => panic!(
                "hart: {}, interrupt {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
                get_hartid(),
                scause.cause(),
                stval,
                cx.sepc
            ),
        },
    }
}

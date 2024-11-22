//! trap handler

use alloc::sync::Arc;

use riscv::register::{
    scause::{self, Exception, Trap},
    stval,
};

use super::trap::set_kernel_trap_entry;
use crate::{constant::register::A0, syscall::syscall, task::Task};

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    panic!("a trap in kernel");
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    info!("call: trap handler");
    set_kernel_trap_entry();
    let mut cx = task.trap_context_mut();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        // syscall
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            let result = syscall(task, cx).await;
            trace!("syscall done! result {:#x}", result);
            cx = task.trap_context_mut();
            cx.regs[A0] = result as usize;
        }
        _ => panic!(
            "trap {:?} is unsupported, stval = {:#x}!",
            scause.cause(),
            stval
        ),
    }
}

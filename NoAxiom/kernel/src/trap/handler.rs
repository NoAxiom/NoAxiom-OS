//! trap handler

use alloc::sync::Arc;

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, stval,
    stvec::{self, TrapMode},
};

use crate::{
    arch::regs::gpr_const::A0, cpu::current_cpu, mm::VirtAddr, syscall::syscall, task::Task,
};

/// set trap entry in smode
fn set_kernel_trap_entry() {
    unsafe { stvec::write(super::trap_from_kernel as usize, TrapMode::Direct) }
}

/// set trap entry in umode
fn set_user_trap_entry() {
    unsafe { stvec::write(super::user_trapvec as usize, TrapMode::Direct) }
}

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    panic!("a trap in kernel");
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    set_kernel_trap_entry();
    let mut cx = task.trap_context_mut();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        // 陷入：系统调用请求
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

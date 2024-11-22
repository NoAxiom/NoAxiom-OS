pub mod context;
pub mod handler;

use alloc::sync::Arc;
use core::arch::global_asm;

use context::TrapContext;
use log::debug;
use riscv::register::{
    sstatus,
    stvec::{self, TrapMode},
};

use crate::{
    arch::interrupt::{
        enable_stimer_interrupt, external_interrupt_enable, interrupt_disable, interrupt_enable,
    },
    println,
    task::Task,
};

global_asm!(include_str!("./trap.S"));
extern "C" {
    fn user_trapvec();
    fn user_trapret(cx: *mut TrapContext);
    fn trap_from_kernel();
}

/// trap init
pub fn init() {
    external_interrupt_enable();
    set_kernel_trap_entry();
    enable_stimer_interrupt();
}

/// set trap entry in supervisor mode
fn set_kernel_trap_entry() {
    unsafe { stvec::write(trap_from_kernel as usize, TrapMode::Direct) }
}

/// set trap entry in user mode
fn set_user_trap_entry() {
    unsafe { stvec::write(user_trapvec as usize, TrapMode::Direct) }
}
pub fn show_sstatus() {
    println!("show sstatus");
    let sstatus = sstatus::read();
    let spie = sstatus.spie(); // 保存的是上一次的sie值，之后通过此位来恢复sie位值
    let sie = sstatus.sie();
    println!("spie:{:?}", spie);
    println!("sie:{:?}", sie);
}

#[no_mangle]
/// kernel back to user
pub fn trap_return(task: &Arc<Task>) {
    set_user_trap_entry(); // 设置用户态下的 trap 入口，当 user to kernel 返回时恢复内核中任务的上下文信息

    let cx = task.trap_context_mut();
    info!("trap return sepc {:#x}", cx.sepc);
    info!("trap return, sp {:#x}", cx.regs[2]);

    // task.timeinfo_mut().record_kernel_to_user();
    // take_current_trap_cx().freg.restore(); // 浮点寄存器的保存
    // unsafe { sstatus::set_fs(sstatus::FS::Clean) };

    // kernel -> user
    // interrupt_enable();
    unsafe {
        user_trapret(task.trap_context_mut());
    }
    // interrupt_disable();

    info!("trap return done");

    // take_current_trap_cx()
    //     .freg
    //     .mark_save_if_needed(take_current_trap_cx().sstatus);

    // task.timeinfo_mut().record_user_to_kernel();
}

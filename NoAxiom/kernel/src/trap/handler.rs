//! trap handler

use alloc::sync::Arc;

use plic::Mode;
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, stval,
};

use super::trap::set_kernel_trap_entry;
#[cfg(feature = "async_fs")]
use crate::driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK;
use crate::{
    config::fs::WAKE_NUM, constant::register::A0, cpu::get_hartid, platform::plic::PLIC,
    sched::utils::yield_now, syscall::syscall, task::Task,
};

fn ext_int_handler() {
    #[cfg(feature = "async_fs")]
    {
        let plic = PLIC.get().unwrap();
        let irq = plic.claim(get_hartid() as u32, Mode::Supervisor);
        debug!("[SupervisorExternal] hart: {}, irq: {}", get_hartid(), irq);
        unsafe {
            VIRTIO_BLOCK
                .0
                .handle_interrupt()
                .expect("virtio handle interrupt error!")
        };
        VIRTIO_BLOCK.0.wake_ops.notify(WAKE_NUM);
        plic.complete(get_hartid() as u32, Mode::Supervisor, irq);
        debug!("[SupervisorExternal] plic complete done!");
    }
}

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    let scause = scause::read();
    let stval = stval::read();
    let sepc = sepc::read();
    match scause.cause() {
        Trap::Exception(exception) => match exception {
            _ => panic!(
                "hart: {}, kernel exception {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
                get_hartid(),
                scause.cause(),
                stval,
                sepc
            ),
        },
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorExternal => {
                #[cfg(feature = "async_fs")]
                {
                    let plic = PLIC.get().unwrap();
                    let irq = plic.claim(get_hartid() as u32, Mode::Supervisor);
                    debug!("[SupervisorExternal] hart: {}, irq: {}", get_hartid(), irq);
                    unsafe {
                        VIRTIO_BLOCK
                            .0
                            .handle_interrupt()
                            .expect("virtio handle interrupt error!")
                    };
                    VIRTIO_BLOCK.0.wake_ops.notify(WAKE_NUM);
                    plic.complete(get_hartid() as u32, Mode::Supervisor, irq);
                }
                #[cfg(not(feature = "async_fs"))]
                {
                    panic!(
                        "hart: {}, kernel SupervisorExternal interrupt is unsupported, stval = {:#x}, sepc = {:#x}",
                        get_hartid(),
                        stval,
                        sepc
                    )
                }
            }
            _ => panic!(
                "hart: {}, kernel interrupt {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
                get_hartid(),
                scause.cause(),
                stval,
                sepc
            ),
        },
    }
    // panic!(
    //     "kernel trap!!! trap {:?} is unsupported, stval = {:#x}, error pc =
    // {:#x}",     scause.cause(),
    //     stval,
    //     sepc,
    // );
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    trace!("[trap_handler] call trap handler");
    set_kernel_trap_entry();
    let mut cx = task.trap_context_mut();
    let scause = scause::read();
    let stval = stval::read();
    trace!(
        "[trap_handler] handle begin, scause: {:?}, stval: {:#x}",
        scause.cause(),
        stval
    );
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
                trace!(
                    "[SupervisorTimer] hart: {}, tid: {}",
                    get_hartid(),
                    task.tid(),
                );
                yield_now().await;
            }
            Interrupt::SupervisorExternal => {
                trace!(
                    "[SupervisorExternal] interrupted at hart: {}, tid: {}",
                    get_hartid(),
                    task.tid(),
                );
                ext_int_handler();
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

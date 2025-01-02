//! trap handler

use alloc::sync::Arc;

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, stval,
};

use super::trap::set_kernel_trap_entry;
use crate::{
    constant::register::A0,
    cpu::{current_cpu, get_hartid},
    sched::utils::yield_now,
    syscall::syscall,
    task::Task,
};

fn ext_int_handler() {
    #[cfg(feature = "async_fs")]
    {
        use plic::Mode;

        use crate::{
            config::fs::WAKE_NUM, driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK,
            platform::plic::PLIC,
        };

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
    let kernel_panic = || {
        panic!(
            "kernel trap!!! trap {:?} is unsupported, stval = {:#x}, error pc = {:#x}",
            scause.cause(),
            stval,
            sepc
        );
    };
    match scause.cause() {
        Trap::Exception(exception) => match exception {
            Exception::LoadPageFault
            | Exception::StorePageFault
            | Exception::InstructionPageFault => {
                if let Some(task) = current_cpu().task.as_mut() {
                    match task.memory_validate(stval) {
                        Ok(_) => trace!("[memory_validate] success in kernel_trap_handler"),
                        Err(_) => kernel_panic(),
                    }
                } else {
                    kernel_panic();
                }
            }
            _ => kernel_panic(),
        },
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorExternal => {
                #[cfg(feature = "async_fs")]
                {
                    use plic::Mode;

                    use crate::{
                        config::fs::WAKE_NUM, driver::async_virtio_driver::virtio_mm::VIRTIO_BLOCK,
                        platform::plic::PLIC,
                    };

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
            _ => kernel_panic(),
        },
    }
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
        "[user_trap_handler] handle begin, scause: {:?}, stval: {:#x}",
        scause.cause(),
        stval
    );
    // for debug, print current error message and exit the task
    let user_exit = || {
        error!("[user_trap_handler] unexpected exit!!! tid: {}, hart: {}, cause: {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
            task.tid(),
            get_hartid(),
            scause.cause(),
            stval,
            cx.sepc
        );
        task.exit();
    };
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
            // page fault: try to handle copy-on-write, or exit the task
            Exception::LoadPageFault
            | Exception::StorePageFault
            | Exception::InstructionPageFault => match task.memory_validate(stval) {
                Ok(_) => trace!("[memory_validate] success in user_trap_handler"),
                Err(_) => {
                    error!(
                        "[user_trap] page fault at hart: {}, tid: {}",
                        get_hartid(),
                        task.tid()
                    );
                    user_exit()
                }
            },
            _ => {
                user_exit();
            }
        },
        // interrupt
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
            _ => {
                user_exit();
            }
        },
    }
}

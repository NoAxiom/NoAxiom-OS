//! trap handler

use alloc::sync::Arc;

use arch::{Arch, ArchInt, ArchTrap, Exception, Interrupt, Trap};

use super::{ipi::ipi_handler, trap::set_kernel_trap_entry};
use crate::{
    // constant::register::A0,
    cpu::{current_cpu, get_hartid},
    sched::utils::{block_on, yield_now},
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
        // debug!("[SupervisorExternal] hart: {}, irq: {}", get_hartid(), irq);
        unsafe {
            VIRTIO_BLOCK
                .0
                .handle_interrupt()
                .expect("virtio handle interrupt error!");
            assert!(!Arch::is_interrupt_enabled());
            // debug!("virtio handle interrupt done!  Notify begin...");
            VIRTIO_BLOCK.0.wake_ops.notify(WAKE_NUM);
        };
        // debug!("Notify done!");
        plic.complete(get_hartid() as u32, Mode::Supervisor, irq);
        // debug!("plic complete done!");
    }
    #[cfg(not(feature = "async_fs"))]
    {
        let scause = Arch::read_trap_cause(); // scause::read();
        let stval = Arch::read_trap_value(); // stval::read();
        let sepc = Arch::read_trap_pc(); // sepc::read();
        panic!(
            "hart: {}, kernel SupervisorExternal interrupt is unsupported, stval = {:#x}, sepc = {:#x}",
            get_hartid(),
            stval,
            sepc
        )
    }
}

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    let scause = Arch::read_trap_cause(); // scause::read();
    let stval = Arch::read_trap_value(); // stval::read();
    let sepc = Arch::read_trap_pc(); // sepc::read();
    let kernel_panic = |msg: &str| {
        panic!(
            "kernel trap!!! msg: {}, trap {:?} is unsupported, stval = {:#x}, error pc = {:#x}",
            msg, scause, stval, sepc
        );
    };
    match scause {
        Trap::Exception(exception) => match exception {
            Exception::LoadPageFault
            | Exception::StorePageFault
            | Exception::InstructionPageFault => {
                if let Some(task) = current_cpu().task.as_mut() {
                    // fixme: currently this block_on cannot be canceled
                    match block_on(task.memory_validate(stval, Some(exception))) {
                        Ok(_) => trace!("[memory_validate] success in kernel_trap_handler"),
                        Err(_) => kernel_panic("memory_validate failed"),
                    }
                } else {
                    kernel_panic("page fault without task running");
                }
            }
            _ => kernel_panic("unsupported exception"),
        },
        Trap::Interrupt(interrupt) => match interrupt {
            Interrupt::SupervisorExternal => {
                ext_int_handler();
            }
            Interrupt::SupervisorTimer => {
                trace!("[SupervisorTimer] kernel Timer");
                // fixme: now is just reset timer
                crate::time::timer::set_next_trigger();
            }
            Interrupt::SupervisorSoft => {
                ipi_handler();
            }
            _ => kernel_panic("unsupported interrupt"),
        },
    }
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    assert!(!Arch::is_interrupt_enabled());
    trace!("[trap_handler] call trap handler");
    set_kernel_trap_entry();
    let cx = task.trap_context_mut();
    let scause = Arch::read_trap_cause(); // scause::read();
    let stval = Arch::read_trap_value(); // stval::read();
    trace!(
        "[user_trap_handler] handle begin, scause: {:?}, stval: {:#x}",
        scause,
        stval
    );
    // for debug, print current error message and exit the task
    let user_exit = |msg: &str| {
        error!("[user_trap_handler] unexpected exit!!! msg: {}, tid: {}, hart: {}, cause: {:?} is unsupported, stval = {:#x}, sepc = {:#x}",
            msg,
            task.tid(),
            get_hartid(),
            scause,
            stval,
            cx.sepc
        );
        task.exit(-1);
    };
    match scause {
        // syscall
        Trap::Exception(exception) => match exception {
            Exception::UserEnvCall => {
                cx.sepc += 4;
                trace!("[syscall] doing syscall");
                let result = syscall(task, cx).await;
                trace!("[syscall] done! result {:#x}", result);
                task.trap_context_mut().set_result(result as usize);
            }
            // page fault: try to handle copy-on-write, or exit the task
            Exception::LoadPageFault
            | Exception::StorePageFault
            | Exception::InstructionPageFault => {
                match task.memory_validate(stval, Some(exception)).await {
                    Ok(_) => trace!("[memory_validate] success in user_trap_handler"),
                    Err(_) => {
                        error!(
                            "[user_trap] page fault at hart: {}, tid: {}",
                            get_hartid(),
                            task.tid()
                        );
                        user_exit("memory_validate failed");
                    }
                }
            }
            _ => {
                user_exit("unsupported exception");
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
            Interrupt::SupervisorSoft => {
                trace!(
                    "[SupervisorSoft] interrupted at hart: {}, tid: {}",
                    get_hartid(),
                    task.tid(),
                );
                ipi_handler();
            }
            _ => {
                user_exit("unsupported interrupt");
            }
        },
    }
}

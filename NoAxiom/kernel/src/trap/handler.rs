//! trap handler

use alloc::{borrow::ToOwned, sync::Arc};

use arch::{Arch, ArchBoot, ArchInt, ArchTrap, TrapArgs, TrapType};

use super::{ext_int::ext_int_handler, ipi::ipi_handler};
use crate::{
    cpu::{current_cpu, get_hartid},
    sched::utils::block_on,
    task::Task,
};

/// kernel trap handler
#[no_mangle]
fn kernel_trap_handler() {
    let trap_type = Arch::read_trap_type(None);
    let epc = Arch::read_epc();
    let kernel_panic = |msg: &str| {
        panic!(
            "kernel trap!!! msg: {}, trap_type: {:#x?}, epc: {:#x} ",
            msg, trap_type, epc,
        );
    };
    match trap_type {
        TrapType::StorePageFault(addr)
        | TrapType::LoadPageFault(addr)
        | TrapType::InstructionPageFault(addr) => {
            if let Some(task) = current_cpu().task.as_mut() {
                // fixme: currently this block_on cannot be canceled
                trace!(
                    "[kernel] block on memory_validate, epc: {:#x}, addr: {:#x}",
                    epc,
                    addr
                );
                match block_on(task.memory_validate(addr, Some(trap_type), true)) {
                    Ok(_) => trace!("[memory_validate] success in kernel_trap_handler"),
                    Err(_) => kernel_panic("memory_validate failed"),
                }
            } else {
                kernel_panic("page fault without task running");
            }
        }
        TrapType::SupervisorExternal => ext_int_handler(),
        TrapType::Timer => {
            // trace!("[SupervisorTimer] kernel Timer");
            // fixme: now is just reset timer
            crate::time::timer::set_next_trigger();
        }
        TrapType::SupervisorSoft => ipi_handler(),
        TrapType::None => {}
        _ => kernel_panic("unsupported trap type"),
    }
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>) {
    assert!(!Arch::is_interrupt_enabled());
    trace!("[trap_handler] call trap handler");
    Arch::set_kernel_trap_entry();
    let cx = task.trap_context_mut();
    let epc = Arch::read_epc();
    let trap_type = Arch::read_trap_type(Some(cx));
    let user_exit = |msg: &str| {
        panic!(
            "[user_trap_handler] unexpected exit!!! msg: {}, trap_type: {:#x?}, sepc = {:#x}",
            msg, trap_type, epc
        );
        task.terminate(-1);
    };
    match trap_type {
        // syscall
        TrapType::SysCall => {
            cx[TrapArgs::EPC] += 4;
            let result = task.syscall(cx).await;
            trace!("[syscall] done! result {:#x}", result);
            task.trap_context_mut()[TrapArgs::RES] = result as usize;
        }
        // page fault: try to handle copy-on-write, or exit the task
        TrapType::LoadPageFault(addr)
        | TrapType::StorePageFault(addr)
        | TrapType::InstructionPageFault(addr) => {
            match task.memory_validate(addr, Some(trap_type), false).await {
                Ok(_) => trace!("[memory_validate] success in user_trap_handler"),
                Err(_) => {
                    panic!(
                        "[user_trap] page fault at hart: {}, tid: {}, epc = {:#x}, addr = {:#x}, user_sp = {:#x}",
                        get_hartid(),
                        task.tid(),
                        cx[TrapArgs::EPC],
                        addr,
                        cx[TrapArgs::SP],
                    );
                    user_exit("memory_validate failed");
                }
            }
        }
        // interrupt
        TrapType::Timer => {
            trace!(
                "[SupervisorTimer] hart: {}, tid: {}",
                get_hartid(),
                task.tid(),
            );
            task.yield_now().await;
        }
        TrapType::SupervisorExternal => {
            trace!(
                "[SupervisorExternal] interrupted at hart: {}, tid: {}",
                get_hartid(),
                task.tid(),
            );
            ext_int_handler();
        }
        TrapType::SupervisorSoft => {
            trace!(
                "[SupervisorSoft] interrupted at hart: {}, tid: {}",
                get_hartid(),
                task.tid(),
            );
            ipi_handler();
        }
        TrapType::None => {}
        _ => {
            user_exit("unsupported trap type");
        }
    }
}

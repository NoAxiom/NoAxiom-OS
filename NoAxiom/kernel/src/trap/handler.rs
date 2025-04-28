//! trap handler

use alloc::sync::Arc;

use arch::{Arch, ArchInt, ArchTrap, TrapArgs, TrapType};

use super::{ext_int::ext_int_handler, ipi::ipi_handler};
use crate::{
    cpu::{current_cpu, current_task, get_hartid},
    sched::utils::{block_on, yield_now},
    signal::{
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    task::Task,
};

/// kernel trap handler
#[no_mangle]
fn kernel_trap_handler() {
    let trap_type = Arch::read_trap_type(None);
    let epc = Arch::read_epc();
    let kernel_panic = |msg: &str| {
        error!(
            "[kernel trap] msg: {}, trap_type: {:#x?}, epc: {:#x} ",
            msg, trap_type, epc,
        );
        let cx = current_task().trap_context();
        error!("[kernel trap] cx: {:#x?}", cx);
        panic!();
    };
    match trap_type {
        TrapType::StorePageFault(addr)
        | TrapType::LoadPageFault(addr)
        | TrapType::InstructionPageFault(addr) => {
            if let Some(task) = current_cpu().task.as_mut() {
                // fixme: currently this block_on cannot be canceled
                info!(
                    "[kernel] block on memory_validate, epc: {:#x}, addr: {:#x}",
                    epc, addr
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
            crate::time::time_slice::set_next_trigger();
        }
        TrapType::SupervisorSoft => ipi_handler(),
        TrapType::None => {}
        _ => kernel_panic("unsupported trap type"),
    }
}

/// user trap handler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>, trap_type: TrapType) {
    assert!(!Arch::is_interrupt_enabled());
    trace!("[trap_handler] call trap handler");

    // check if need schedule
    if task.tcb().time_stat.is_timeup() {
        trace!(
            "task {} yield by time = {:?}",
            task.tid(),
            task.tcb().time_stat,
        );
        yield_now().await;
    }

    // def: context, user trap pc, trap type
    let cx = task.trap_context_mut();

    // user exit when detect unexpected trap
    let user_exit = |msg: &str| {
        error!(
            "[user_trap_handler] unexpected exit!!! msg: {}, trap_type: {:#x?}, sepc = {:#x}, cx = {:#x?}",
            msg,
            trap_type,
            cx[TrapArgs::EPC],
            cx,
        );
        task.recv_siginfo(
            &mut task.pcb(),
            SigInfo::new_simple(SigNum::SIGSEGV.into(), SigCode::Kernel),
            false,
        );
    };

    // user trap handler vector
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
                    error!(
                        "[user_trap] page fault at hart: {}, tid: {}, epc = {:#x}, addr = {:#x}, user_sp = {:#x}, ra = {:#x}",
                        get_hartid(),
                        task.tid(),
                        cx[TrapArgs::EPC],
                        addr,
                        cx[TrapArgs::SP],
                        cx[TrapArgs::RA],
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

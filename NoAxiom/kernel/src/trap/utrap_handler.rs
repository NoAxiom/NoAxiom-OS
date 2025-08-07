//! user trap handler
//! [`user_trap_handler`] is used by [`arch`]

use alloc::sync::Arc;
use core::intrinsics::unlikely;

use arch::{
    consts::KERNEL_ADDR_OFFSET, Arch, ArchInt, ExceptionType, InterruptType, PageFaultType,
    TrapArgs, TrapType,
};

use super::{ext_int::ext_int_handler, soft_int::soft_int_handler};
use crate::{
    fs::vfs::inc_interrupts_count,
    signal::{
        sig_info::{SigCode, SigInfo},
        signal::Signal,
    },
    task::Task,
    time::time_slice::set_next_trigger,
};

/// user trap handler
/// WARNING: don't try to use nested async function here
/// or it would lead to data inconsistency caused by LA compiler
#[no_mangle]
pub async fn user_trap_handler(task: &Arc<Task>, trap_type: TrapType) {
    Arch::disable_interrupt();
    trace!("[trap_handler] call trap handler");

    // check if need schedule
    if unlikely(task.need_resched()) {
        trace!(
            "task {} time_stat timeup by time = {:?}",
            task.tid(),
            task.time_stat(),
        );
        task.yield_now().await;
    }

    // def: context, user trap pc, trap type
    let cx = task.trap_context_mut();
    assert!(
        cx[TrapArgs::EPC] & KERNEL_ADDR_OFFSET == 0,
        "epc {:#x?} shouldn't be in kernel space, trap_type: {:x?}",
        cx[TrapArgs::EPC],
        trap_type
    );

    // user trap handler vector
    match trap_type {
        TrapType::Exception(exc) => match exc {
            ExceptionType::Syscall => {
                Arch::enable_interrupt();
                let result = task.syscall(cx).await;
                task.update_syscall_result(result);
            }
            ExceptionType::PageFault(pf) => {
                match pf {
                    // page fault: try to handle copy-on-write, or exit the task
                    PageFaultType::LoadPageFault(addr)
                    | PageFaultType::StorePageFault(addr)
                    | PageFaultType::InstructionPageFault(addr) => {
                        match task.memory_validate(addr, pf, false).await {
                            Ok(_) => trace!("[memory_validate] success in user_trap_handler"),
                            Err(_) => {
                                warn!(
                                    "[user_trap] pagefault @ {:#x}, tid: {}, trap_type: {:x?}, epc = {:#x}, user_sp = {:#x}, ra = {:#x}",
                                    addr,
                                    task.tid(),
                                    pf,
                                    cx[TrapArgs::EPC],
                                    cx[TrapArgs::SP],
                                    cx[TrapArgs::RA],
                                );
                                task.recv_siginfo(
                                    SigInfo::new_simple(Signal::SIGSEGV.into(), SigCode::Kernel),
                                    true,
                                );
                            }
                        }
                    }
                    PageFaultType::IllegalInstruction(addr) => {
                        warn!(
                            "[user_trap] illegal instruction @ {:#x}, tid: {}, trap_type: {:x?}, epc = {:#x}, user_sp = {:#x}, ra = {:#x}",
                            addr,
                            task.tid(),
                            trap_type,
                            cx[TrapArgs::EPC],
                            cx[TrapArgs::SP],
                            cx[TrapArgs::RA],
                        );
                        task.recv_siginfo(
                            SigInfo::new_simple(Signal::SIGILL.into(), SigCode::Kernel),
                            true,
                        );
                    }
                }
            }
            ExceptionType::Breakpoint => {
                warn!(
                    "[user_trap] breakpoint exception, tid: {}, trap_type: {:x?}, epc = {:#x}, user_sp = {:#x}, ra = {:#x}",
                    task.tid(),
                    trap_type,
                    cx[TrapArgs::EPC],
                    cx[TrapArgs::SP],
                    cx[TrapArgs::RA],
                );
                task.recv_siginfo(
                    SigInfo::new_simple(Signal::SIGTRAP.into(), SigCode::Kernel),
                    true,
                );
            }
        },
        TrapType::Interrupt(int) => {
            match int {
                // interrupt
                InterruptType::Timer(id) => {
                    // trace!(
                    //     "[SupervisorTimer] hart: {}, tid: {}",
                    //     get_hartid(),
                    //     task.tid(),
                    // );
                    inc_interrupts_count(id);
                    set_next_trigger(None);
                    task.yield_now().await;
                }
                InterruptType::SupervisorExternal(id) => {
                    // trace!(
                    //     "[SupervisorExternal] interrupted at hart: {}, tid: {}",
                    //     get_hartid(),
                    //     task.tid(),
                    // );
                    inc_interrupts_count(id);
                    ext_int_handler();
                }
                InterruptType::SupervisorSoft(id) => {
                    // trace!(
                    //     "[SupervisorSoft] interrupted at hart: {}, tid: {}",
                    //     get_hartid(),
                    //     task.tid(),
                    // );
                    inc_interrupts_count(id);
                    soft_int_handler();
                }
            };
        }
        TrapType::None | TrapType::Handled => {}
        TrapType::Unknown => {
            panic!("unsupported trap type: {trap_type:x?}");
        }
    }
}

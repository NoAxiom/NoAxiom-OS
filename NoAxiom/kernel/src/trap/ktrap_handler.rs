//! kernel trap handler
//! [`kernel_trap_handler`] is used by [`arch`]

use arch::{Arch, ArchInt, ArchTime, ExceptionType, InterruptType, PageFaultType, TrapType};
use kfuture::block::block_on;

use crate::{
    cpu::{current_cpu, current_task},
    fs::vfs::inc_interrupts_count,
    syscall::utils::current_syscall,
    trap::{ext_int::ext_int_handler, soft_int::soft_int_handler},
};

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler(trap_type: &TrapType) {
    assert!(!Arch::is_interrupt_enabled());
    current_cpu().add_trap_depth();
    match *trap_type {
        TrapType::Exception(exception) => kernel_exception_handler(exception),
        TrapType::Interrupt(interrupt) => kernel_interrupt_handler(interrupt),
        TrapType::None => {}
        TrapType::Unknown => panic!("unsupported trap type"),
    }
    current_cpu().sub_trap_depth();
}

fn kernel_exception_handler(exception: ExceptionType) {
    match exception {
        ExceptionType::PageFault(pf) => match pf {
            PageFaultType::StorePageFault(addr)
            | PageFaultType::LoadPageFault(addr)
            | PageFaultType::InstructionPageFault(addr) => {
                let task = current_cpu().task.as_mut();
                if let Some(task) = task {
                    // fixme: currently this block_on cannot be canceled
                    warn!(
                        "[kernel] block on memory_validate, addr: {:#x}, syscall: {:?}",
                        addr,
                        current_syscall(),
                    );
                    match block_on(task.memory_validate(addr, pf, true)) {
                        Ok(_) => trace!("[memory_validate] success in kernel_trap_handler"),
                        Err(_) => panic!("memory_validate failed"),
                    }
                } else {
                    panic!("page fault without task running, addr: {:#x}", addr);
                }
            }
            _ => panic!("unsupported page fault type: {:?}", pf),
        },
        _ => panic!("unsupported exc type: {:?}", exception),
    }
}

fn kernel_interrupt_handler(interrupt: InterruptType) {
    use InterruptType::*;
    match interrupt {
        SupervisorExternal(id) => {
            inc_interrupts_count(id);
            ext_int_handler()
        }
        Timer(id) => {
            inc_interrupts_count(id);
            kernel_timer_trap_handler()
        }
        SupervisorSoft(id) => {
            inc_interrupts_count(id);
            soft_int_handler()
        }
    }
}

pub fn kernel_timer_trap_handler() {
    // mark the task as needing to yield
    if let Some(task) = current_task() {
        task.sched_entity_mut().set_pending_yield();
    }
    Arch::clear_timer_interrupt();
    // if current_cpu().trap_depth() < 2 {
    //     TIMER_MANAGER.check();
    //     RUNTIME.handle_realtime();
    // }
}

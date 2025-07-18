use arch::{Arch, ArchTrap, ExceptionType, InterruptType, PageFaultType, TrapType};
use kfuture::block::block_on;

use crate::{
    cpu::current_cpu,
    syscall::utils::current_syscall,
    trap::{ext_int::ext_int_handler, ipi::ipi_handler, ktimer::kernel_timer_trap_handler},
};

/// kernel trap handler
#[no_mangle]
pub fn kernel_trap_handler() {
    current_cpu().add_trap_depth();
    let trap_type = Arch::read_trap_type(None);
    match trap_type {
        TrapType::Exception(exception) => kernel_exception_handler(exception),
        TrapType::Interrupt(interrupt) => kernel_interrupt_handler(interrupt),
        TrapType::None => {}
        _ => panic!("unsupported trap type"),
    }
    current_cpu().sub_trap_depth();
}

fn kernel_exception_handler(exception: ExceptionType) {
    match exception {
        ExceptionType::PageFault(pf) => match pf {
            PageFaultType::StorePageFault(addr)
            | PageFaultType::LoadPageFault(addr)
            | PageFaultType::InstructionPageFault(addr) => {
                let task = current_cpu()
                    .task
                    .as_mut()
                    .expect("page fault without task running");
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
            }
            _ => panic!("unsupported page fault type: {:?}", pf),
        },
        _ => panic!("unsupported exc type: {:?}", exception),
    }
}

fn kernel_interrupt_handler(interrupt: InterruptType) {
    use InterruptType::*;
    match interrupt {
        SupervisorExternal(id) => ext_int_handler(),
        Timer(id) => kernel_timer_trap_handler(),
        SupervisorSoft(id) => ipi_handler(),
    }
}

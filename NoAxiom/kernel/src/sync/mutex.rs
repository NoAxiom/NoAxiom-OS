//! spin mutex for riscv kernel

use core::cell::{RefCell, RefMut};

use kernel_sync::{ticket::TicketMutexGuard, LockAction};

use super::cell::SyncRefCell;
use crate::{
    arch::interrupt::{disable_global_interrupt, enable_global_interrupt, is_interrupt_enabled},
    config::arch::CPU_NUM,
    cpu::get_hartid,
};

pub type SpinMutex<T> = kernel_sync::spin::SpinMutex<T, IrqOffLockAction>;
pub type TicketMutex<T> = kernel_sync::ticket::TicketMutex<T, IrqOffLockAction>;
pub type NoIrqSpinMutex<T> = kernel_sync::spin::SpinMutex<T, NoIrqLockAction>;
pub type NoIrqTicketMutex<T> = kernel_sync::ticket::TicketMutex<T, NoIrqLockAction>;

#[derive(Debug, Default, Clone, Copy)]
#[repr(align(64))]
struct MutexTracer {
    pub depth: i32,
    pub int_record: bool,
}
impl MutexTracer {
    const fn new() -> Self {
        Self {
            depth: 0,
            int_record: false,
        }
    }
}

#[allow(clippy::declare_interior_mutable_const)]
const DEFAULT_CPU: SyncRefCell<MutexTracer> = SyncRefCell::new(MutexTracer::new());
static HART_MUTEX_TRACERS: [SyncRefCell<MutexTracer>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];
fn current_mutex_tracer() -> RefMut<'static, MutexTracer> {
    HART_MUTEX_TRACERS[get_hartid()].borrow_mut()
}

/// maintain riscv arch interrupt behavior for lock action
pub struct NoIrqLockAction;
impl LockAction for NoIrqLockAction {
    fn before_lock() {
        let old = is_interrupt_enabled();
        disable_global_interrupt();
        let mut cpu = current_mutex_tracer();
        if cpu.depth == 0 {
            cpu.int_record = old;
        }
        cpu.depth += 1;
    }
    fn after_lock() {
        let mut cpu = current_mutex_tracer();
        cpu.depth -= 1;
        let should_enable = cpu.depth == 0 && cpu.int_record;
        drop(cpu); // drop before int_en
        if should_enable {
            enable_global_interrupt();
        }
    }
}

/// assert irq is off
pub struct IrqOffLockAction;
impl LockAction for IrqOffLockAction {
    fn before_lock() {
        assert!(!is_interrupt_enabled());
    }
    fn after_lock() {
        assert!(!is_interrupt_enabled());
    }
}

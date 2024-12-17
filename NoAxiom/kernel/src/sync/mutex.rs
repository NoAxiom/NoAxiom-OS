//! spin mutex for riscv kernel

use core::cell::{RefCell, RefMut};

use kernel_sync::{ticket::TicketMutexGuard, LockAction};

use super::cell::SyncRefCell;
use crate::{
    arch::interrupt::{disable_global_interrupt, enable_global_interrupt, is_interrupt_enabled},
    config::arch::CPU_NUM,
    cpu::get_hartid,
};

pub type SpinMutex<T> = kernel_sync::spin::SpinMutex<T, KernelLockAction>;
pub type TicketMutex<T> = kernel_sync::ticket::TicketMutex<T, KernelLockAction>;
pub type RwLock<T> = kernel_sync::RwLock<T>;
pub type Mutex<T> = TicketMutex<T>;
pub type MutexGuard<'a, T> = TicketMutexGuard<'a, T, KernelLockAction>;

#[derive(Debug, Default, Clone, Copy)]
#[repr(align(64))]
struct MutexTracer {
    pub depth: i32,       // Depth of push_off() nesting.
    pub int_record: bool, // Were interrupts enabled before push_off()?
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

static CPUS: [SyncRefCell<MutexTracer>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

fn current_mutex_tracer() -> RefMut<'static, MutexTracer> {
    CPUS[get_hartid()].borrow_mut()
}

/// provides riscv arch interrupt behavior for lock action
pub struct KernelLockAction;

impl LockAction for KernelLockAction {
    fn before_lock() {
        // assert!(!is_interrupt_enabled());
        let old = is_interrupt_enabled();
        disable_global_interrupt();
        let mut cpu = current_mutex_tracer();
        if cpu.depth == 0 {
            cpu.int_record = old;
        }
        cpu.depth += 1;
    }
    fn after_lock() {
        // assert!(!is_interrupt_enabled());
        let mut cpu = current_mutex_tracer();
        cpu.depth -= 1;
        let should_enable = cpu.depth == 0 && cpu.int_record;
        drop(cpu); // drop before int_en
        if should_enable {
            enable_global_interrupt();
        }
    }
}

/// disable interrupt
pub(crate) fn push_off() {
    // assert!(!is_interrupt_enabled());
    // let old = is_interrupt_enabled();
    // disable_global_interrupt();
    // let mut cpu = current_cpu();
    // if cpu.push_off_depth == 0 {
    //     cpu.interrupt_enable = old;
    // }
    // cpu.push_off_depth += 1;
}

/// enable interrupt if depth decline to 0
pub(crate) fn pop_off() {
    // assert!(!is_interrupt_enabled());
    // let mut cpu = current_cpu();
    // cpu.push_off_depth -= 1;
    // let should_enable = cpu.push_off_depth == 0 && cpu.interrupt_enable;
    // drop(cpu); // drop before int_en
    // if should_enable {
    //     enable_global_interrupt();
    // }
}

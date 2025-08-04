//! spin mutex for riscv kernel

use arch::{Arch, ArchAsm, ArchInt};
use config::cpu::CPU_NUM;

use crate::cell::SyncUnsafeCell;

type LockActionPolicy = NoIrqLockAction;
pub type SpinLock<T> = kernel_sync::spin::SpinMutex<T, LockActionPolicy>;
pub type SpinLockGuard<'a, T> = kernel_sync::spin::SpinMutexGuard<'a, T, LockActionPolicy>;
pub type TicketLock<T> = kernel_sync::ticket::TicketMutex<T, LockActionPolicy>;
pub type RwLock<T> = kernel_sync::rwlock::RwLock<T, LockActionPolicy>;
pub type RwLockReadGuard<'a, T> = kernel_sync::rwlock::RwLockReadGuard<'a, T, LockActionPolicy>;
pub type RwLockWriteGuard<'a, T> = kernel_sync::rwlock::RwLockWriteGuard<'a, T, LockActionPolicy>;

pub type RawSpinLock<T, L> = kernel_sync::spin::SpinMutex<T, L>;
pub type RawSpinLockGuard<'a, T, L> = kernel_sync::spin::SpinMutexGuard<'a, T, L>;

pub use kernel_sync::LockAction;

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

// #[allow(clippy::declare_interior_mutable_const)]
const DEFAULT_CPU: SyncUnsafeCell<MutexTracer> = SyncUnsafeCell::new(MutexTracer::new());
static mut HART_MUTEX_TRACERS: [SyncUnsafeCell<MutexTracer>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];
fn current_mutex_tracer() -> &'static mut MutexTracer {
    unsafe { HART_MUTEX_TRACERS[Arch::get_hartid()].as_ref_mut() }
}

pub fn current_lock_depth() -> usize {
    current_mutex_tracer().depth as usize
}
pub fn check_no_lock() -> bool {
    current_mutex_tracer().depth == 0
}

#[macro_export]
macro_rules! assert_no_lock {
    () => {
        if !ksync::mutex::check_no_lock() {
            panic!(
                "lock assertion failed, current lock depth={}",
                ksync::mutex::current_lock_depth()
            );
        }
    };
}

/// maintain riscv arch interrupt behavior for lock action
pub struct NoIrqLockAction;
impl LockAction for NoIrqLockAction {
    fn before_lock() {
        let old = Arch::is_interrupt_enabled();
        Arch::disable_interrupt();
        let cpu = current_mutex_tracer();
        if cpu.depth == 0 {
            cpu.int_record = old;
        }
        cpu.depth += 1;
    }
    fn after_lock() {
        let cpu = current_mutex_tracer();
        cpu.depth -= 1;
        let should_enable = cpu.depth == 0 && cpu.int_record;
        if should_enable {
            Arch::enable_interrupt();
        }
    }
}

/// assert irq is off
pub struct IrqOffLockAction;
impl LockAction for IrqOffLockAction {
    fn before_lock() {
        assert!(!Arch::is_interrupt_enabled());
    }
    fn after_lock() {
        assert!(!Arch::is_interrupt_enabled());
    }
}

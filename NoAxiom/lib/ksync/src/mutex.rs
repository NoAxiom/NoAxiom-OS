//! spin mutex for riscv kernel

use core::{cell::RefMut, sync::atomic::AtomicUsize};

use arch::{Arch, ArchHart, ArchInt};

use super::cell::SyncRefCell;

pub type SpinLock<T> = Lock<T>;
pub type SpinLockGuard<'a, T> = LockGuard<'a, T>;
// pub type SpinLock<T> = kernel_sync::spin::SpinMutex<T, NoIrqLockAction>;
// pub type SpinLockGuard<'a, T> = kernel_sync::spin::SpinMutexGuard<'a, T,
// NoIrqLockAction>;
pub type TicketLock<T> = kernel_sync::ticket::TicketMutex<T, NoIrqLockAction>;
pub type RwLock<T> = kernel_sync::rwlock::RwLock<T, NoIrqLockAction>;

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

#[allow(clippy::declare_interior_mutable_const)]
const CPU_NUM: usize = 8; // FIXME: use extern const to config cpu_num
const DEFAULT_CPU: SyncRefCell<MutexTracer> = SyncRefCell::new(MutexTracer::new());
static HART_MUTEX_TRACERS: [SyncRefCell<MutexTracer>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];
fn current_mutex_tracer() -> RefMut<'static, MutexTracer> {
    HART_MUTEX_TRACERS[Arch::get_hartid()].borrow_mut()
}

/// maintain riscv arch interrupt behavior for lock action
pub struct NoIrqLockAction;
impl LockAction for NoIrqLockAction {
    fn before_lock() {
        let old = Arch::is_interrupt_enabled();
        Arch::disable_global_interrupt();
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
            Arch::enable_global_interrupt();
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

use spin::{Mutex, MutexGuard};
#[derive(Default)]
pub struct Lock<T>(pub(self) Mutex<T>);

pub static LOCK_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct LockGuard<'a, T> {
    guard: Option<MutexGuard<'a, T>>,
    int_record: bool,
}

impl<T> Lock<T> {
    pub const fn new(obj: T) -> Self {
        Self(Mutex::new(obj))
    }
    pub fn lock(&self) -> LockGuard<'_, T> {
        let old = Arch::is_interrupt_enabled();
        Arch::disable_global_interrupt();
        if Arch::get_hartid() == 0 {
            LOCK_COUNT.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        }
        LockGuard {
            guard: Some(self.0.lock()),
            int_record: old,
        }
    }
}

impl<'a, T> Drop for LockGuard<'a, T> {
    fn drop(&mut self) {
        if Arch::get_hartid() == 0 {
            LOCK_COUNT.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);
        }
        self.guard.take();
        if self.int_record {
            Arch::enable_global_interrupt();
        }
    }
}

impl<'a, T> core::ops::Deref for LockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap().deref()
    }
}

impl<'a, T> core::ops::DerefMut for LockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().unwrap().deref_mut()
    }
}

//! spin mutex for riscv kernel

use core::cell::{RefCell, RefMut};

use kernel_sync::{ticket::TicketMutexGuard, LockAction};

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
struct Cpu {
    pub push_off_depth: i32,    // Depth of push_off() nesting.
    pub interrupt_enable: bool, // Were interrupts enabled before push_off()?
}

impl Cpu {
    const fn new() -> Self {
        Self {
            push_off_depth: 0,
            interrupt_enable: false,
        }
    }
}

pub struct SafeRefCell<T>(RefCell<T>);

/// # Safety: Only the corresponding cpu will access it.
unsafe impl<Cpu> Sync for SafeRefCell<Cpu> {}

impl<T> SafeRefCell<T> {
    const fn new(t: T) -> Self {
        Self(RefCell::new(t))
    }
}

#[allow(clippy::declare_interior_mutable_const)]
const DEFAULT_CPU: SafeRefCell<Cpu> = SafeRefCell::new(Cpu::new());

static CPUS: [SafeRefCell<Cpu>; CPU_NUM] = [DEFAULT_CPU; CPU_NUM];

fn current_cpu() -> RefMut<'static, Cpu> {
    CPUS[get_hartid()].0.borrow_mut()
}

/// provides riscv arch interrupt behavior for lock action
pub struct KernelLockAction;

impl LockAction for KernelLockAction {
    #[inline(always)]
    fn before_lock() {
        push_off();
    }
    #[inline(always)]
    fn after_lock() {
        pop_off();
    }
}

/// disable interrupt
pub(crate) fn push_off() {
    assert!(!is_interrupt_enabled());
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
    assert!(!is_interrupt_enabled());
    // let mut cpu = current_cpu();
    // cpu.push_off_depth -= 1;
    // let should_enable = cpu.push_off_depth == 0 && cpu.interrupt_enable;
    // drop(cpu); // drop before int_en
    // if should_enable {
    //     enable_global_interrupt();
    // }
}

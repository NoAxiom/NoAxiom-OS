//! # async task schedule
//! ## usages
//! - [`utask`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`utils`] contains useful func for async tasks execution

mod executor;
mod utask;
pub mod utils;

use core::sync::atomic::AtomicUsize;

pub use executor::run;
pub use utask::{schedule_spawn_new_process, spawn_task};

pub static mut TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
pub fn task_count_inc() {
    unsafe {
        TASK_COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }
}
pub fn task_count_dec() {
    unsafe {
        TASK_COUNTER.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
    }
    if unsafe { TASK_COUNTER.load(core::sync::atomic::Ordering::Relaxed) == 0 } {
        info!("[kernel] all tasks are done, shutdown");
        crate::driver::sbi::shutdown();
    }
}

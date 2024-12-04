//! task counter, kernel shutdown when TASK_COUNTER decrease to 0

use core::sync::atomic::{AtomicUsize, Ordering};

pub static mut TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn task_count_inc() {
    unsafe {
        TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn task_count_dec() {
    unsafe {
        TASK_COUNTER.fetch_sub(1, Ordering::SeqCst);
    }
    if unsafe { TASK_COUNTER.load(Ordering::Acquire) == 0 } {
        info!("[kernel] all tasks are done, shutdown");
        crate::driver::sbi::shutdown();
    }
}

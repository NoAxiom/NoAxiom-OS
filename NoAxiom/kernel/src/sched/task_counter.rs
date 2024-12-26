//! task counter, kernel shutdown when TASK_COUNTER decrease to 0

use core::sync::atomic::{AtomicUsize, Ordering};

pub static mut TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn task_count_inc() {
    unsafe {
        TASK_COUNTER.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn task_count_dec() {
    if unsafe { TASK_COUNTER.fetch_sub(1, Ordering::SeqCst) } == 1 {
        info!("[kernel] all tasks are done, shutdown");
        error!("shutdown is off, please shutdown the terminal manually");
        // crate::driver::sbi::shutdown();
    }
}

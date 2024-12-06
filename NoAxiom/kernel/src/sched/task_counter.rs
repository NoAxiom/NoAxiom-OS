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
        // print_counter();
        // let logger = unsafe { crate::sched::executor::INFO_LOGGER.lock() };
        // for it in logger.iter() {
        //     debug!("{}", it);
        // }
        crate::driver::sbi::shutdown();
    }
}

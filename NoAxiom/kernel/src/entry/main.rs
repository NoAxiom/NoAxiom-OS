use core::sync::atomic::{AtomicBool, Ordering};

use crate::{cpu::get_hartid, sched::runtime::run_task};

static BOOT_FLAG: AtomicBool = AtomicBool::new(false);

pub fn boot_broadcast() {
    BOOT_FLAG.store(true, Ordering::SeqCst);
}

/// called by [`crate::entry::init::_boot_hart_init`]
/// called by [`crate::entry::init::_other_hart_init`] as well
pub fn rust_main() -> ! {
    while !BOOT_FLAG.load(Ordering::SeqCst) {}
    info!("[kernel] hart {} has been booted", get_hartid());
    loop {
        run_task();
    }
}

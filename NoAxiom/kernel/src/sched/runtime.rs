use lazy_static::lazy_static;

use super::{
    simple::{SimpleRuntime, SimpleScheduler},
    vsched::Runtime,
};
use crate::{cpu::get_hartid, time::time_manager::timer_handler};

lazy_static! {
    pub static ref RUNTIME: SimpleRuntime<SimpleScheduler> = SimpleRuntime::new();
}

// use super::cfs::CfsRuntime;
// use crate::sched::vsched::Runtime;
// lazy_static! {
//     pub(crate) static ref RUNTIME: CfsRuntime = CfsRuntime::new();
// }

/// run_tasks: only act as a task runner
#[no_mangle]
pub fn run_tasks() -> ! {
    info!("[kernel] hart {} has been booted", get_hartid());
    loop {
        timer_handler();
        RUNTIME.run();
    }
}

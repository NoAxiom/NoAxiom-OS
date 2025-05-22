use arch::{Arch, ArchInt, ArchMemory};
use config::task::INIT_PROCESS_ID;
use lazy_static::lazy_static;

use super::vsched::Runtime;
use crate::{
    cpu::{get_hartid, CPUS},
    task::manager::TASK_MANAGER,
    time::{gettime::get_time_duration, timer::timer_handler},
};

type RuntimeImpl = super::simple::SimpleRuntime;
lazy_static! {
    pub static ref RUNTIME: RuntimeImpl = RuntimeImpl::new();
}

/// run_tasks: only act as a task runner
#[no_mangle]
pub fn run_tasks() -> ! {
    info!("[kernel] hart {} has been booted", get_hartid());
    loop {
        assert!(Arch::is_interrupt_enabled());
        timer_handler();
        RUNTIME.run();
        // context_switch_test();
    }
}

#[allow(unused)]
fn context_switch_test() {
    if let Some(init_proc) = TASK_MANAGER.get(INIT_PROCESS_ID) {
        let time0 = get_time_duration();
        const NUM: usize = 100000;
        let mut counter = 0;
        for i in 0..NUM {
            CPUS[get_hartid()].as_ref_mut().set_task(&init_proc);
            counter += i;
            CPUS[get_hartid()].as_ref_mut().clear_task();
        }
        let time1 = get_time_duration();
        for i in 0..NUM {
            Arch::tlb_flush();
            counter += i;
            Arch::tlb_flush();
        }
        let time2 = get_time_duration();
        for i in 0..NUM {
            counter += i;
        }
        let time3 = get_time_duration();
        println!(
            "[kernel] hart {} switch time: {:?}, flush time: {:?}, arith time: {:?}, n: {}, counter: {}",
            get_hartid(),
            time1 - time0,
            time2 - time1,
            time3 - time2,
            NUM,
            counter
        );
    }
}

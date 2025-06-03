use crate::{
    cpu::current_task,
    time::time_slice::{set_next_trigger, TimeSliceInfo},
};

pub fn kernel_timer_trap_handler() {
    // mark the task as needing to yield
    if let Some(task) = current_task() {
        task.sched_entity_mut().set_pending_yield();
    }

    // don't trap again, we've mark yield request to the task
    set_next_trigger(Some(TimeSliceInfo::infinite()));

    // try handle realtime tasks
    // if current_cpu().trap_depth() < 2 {
    //     TIMER_MANAGER.check();
    //     RUNTIME.handle_realtime();
    // } else {
    //     error!(
    //         "[kernel_trap] SupervisorTimer trap at hart: {} with depth > 2,
    // skip check",         get_hartid(),
    //     );
    // }
}

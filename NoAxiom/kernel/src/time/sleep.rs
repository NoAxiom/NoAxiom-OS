use alloc::sync::Arc;
use core::time::Duration;

use super::{
    gettime::get_time_duration,
    time_manager::{Timer, TIMER_MANAGER},
};
use crate::{sched::utils::suspend_now, task::Task};

impl Task {
    /// sleep will suspend the task
    /// but it can be interrupted, so return the time duration
    /// if the result is zero, it indicates the task is woken by sleep event
    pub async fn sleep(self: &Arc<Self>, interval: Duration) -> Duration {
        let expire = get_time_duration() + interval;
        TIMER_MANAGER.add_timer(Timer::new_waker_timer(expire, self.waker().clone()));
        suspend_now(self.pcb()).await;
        let now = get_time_duration();
        if expire > now {
            expire - now
        } else {
            Duration::ZERO
        }
    }
}

pub fn timer_handler() {
    TIMER_MANAGER.check();
}

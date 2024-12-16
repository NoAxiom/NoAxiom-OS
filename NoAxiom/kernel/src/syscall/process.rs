use crate::sched::utils::yield_now;

use super::syscall::Syscall;

impl Syscall<'_> {
    pub fn sys_exit(&mut self) {
        self.task.exit();
        trace!(
            "task exited, tid: {}, counter: {}",
            self.task.tid(),
            unsafe {
                crate::sched::task_counter::TASK_COUNTER.load(core::sync::atomic::Ordering::SeqCst)
            }
        );
    }

    pub async fn sys_yield(&mut self) {
        trace!("sys_yield");
        yield_now().await;
    }
}

use crate::task::{exit::ExitReason, status::TaskStatus, Task};

impl Task {
    /// exit current task
    pub fn terminate(&self, exit_code: ExitReason) {
        let mut pcb = self.pcb();
        if self.is_group_leader() {
            pcb.set_exit_code(exit_code);
        }
        pcb.set_status(TaskStatus::Terminated, self.tif_mut());
    }

    /// terminate all tasks in current thread group
    pub fn terminate_group(&self, exit_code: ExitReason) {
        let tg = self.thread_group();
        for (_id, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.terminate(exit_code);
        }
    }

    /// terminate all tasks except group leader in current thread group
    pub fn terminate_threads(&self) {
        assert!(self.is_group_leader());
        let tg = self.thread_group();
        for (_id, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            if !task.is_group_leader() {
                task.terminate(ExitReason::default());
            }
        }
    }
}

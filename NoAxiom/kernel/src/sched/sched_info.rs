use alloc::sync::Weak;

use super::sched_entity::SchedEntity;
use crate::task::Task;

pub struct SchedInfo {
    pub sched_entity: SchedEntity,
    /// the hartid that the task should be running on
    pub _task: Option<Weak<Task>>,
}
impl SchedInfo {
    pub fn new(sched_entity: SchedEntity, task: Option<Weak<Task>>) -> Self {
        Self {
            sched_entity,
            _task: task,
        }
    }
}

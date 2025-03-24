use alloc::sync::Arc;

use ksync::mutex::SpinLock;

use super::sched_entity::SchedEntity;
use crate::task::{status::TaskStatus, Task};

pub struct SchedInfo {
    pub sched_entity: SchedEntity,
    pub status: Option<Arc<SpinLock<TaskStatus>>>,
}
impl SchedInfo {
    pub fn new(sched_entity: SchedEntity, task: Option<&Arc<Task>>) -> Self {
        let status = task.map(|task| task.status.clone());
        Self {
            sched_entity,
            status,
        }
    }
}

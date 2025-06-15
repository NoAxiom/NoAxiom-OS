use alloc::sync::Arc;

use crate::{include::sched::CpuMask, task::Task, time::time_info::TimeInfo};

#[derive(Debug, Clone, Copy)]
pub enum SchedPrio {
    RealTime(#[allow(unused)] usize),
    Normal,
    IdlePrio,
}

pub struct SchedEntity {
    pub sched_prio: SchedPrio, // scheduling priority
    pub time_stat: TimeInfo,   // task time
    pub cpu_mask: CpuMask,     // cpu mask
    pub yield_req: bool,       // need yield
}

impl Default for SchedEntity {
    fn default() -> Self {
        Self {
            sched_prio: SchedPrio::Normal,
            time_stat: TimeInfo::default(),
            cpu_mask: CpuMask::default(),
            yield_req: false,
        }
    }
}
impl SchedEntity {
    pub fn clear_pending_yield(&mut self) {
        self.yield_req = false;
    }
    pub fn set_pending_yield(&mut self) {
        self.yield_req = true;
    }
    pub fn need_yield(&self) -> bool {
        self.time_stat.is_timeup() || self.yield_req
    }
}

#[derive(Clone, Copy)]
pub struct SchedMetadata {
    ptr: *mut SchedEntity,
    tid: usize,
}

impl SchedMetadata {
    pub fn from_task(task: &Arc<Task>) -> Self {
        Self {
            ptr: task.get_sched_entity(),
            tid: task.tid(),
        }
    }
    pub fn sched_entity(&self) -> Option<&SchedEntity> {
        if self.ptr.is_null() {
            None
        } else {
            unsafe { Some(&*self.ptr) }
        }
    }
    #[allow(unused)]
    pub fn tid(&self) -> usize {
        self.tid
    }
}

impl Default for SchedMetadata {
    fn default() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            tid: 0,
        }
    }
}

unsafe impl Sync for SchedMetadata {}
unsafe impl Send for SchedMetadata {}

use crate::{include::sched::CpuMask, time::time_info::TimeInfo};

#[derive(Debug, Clone, Copy)]
pub enum SchedPrio {
    RealTime(usize),
    Normal,
    IdlePrio,
}

pub struct SchedEntity {
    pub sched_prio: SchedPrio, // scheduling priority
    pub time_stat: TimeInfo,   // task time
    pub cpu_mask: CpuMask,     // cpu mask
}

impl Default for SchedEntity {
    fn default() -> Self {
        Self {
            sched_prio: SchedPrio::Normal,
            time_stat: TimeInfo::default(),
            cpu_mask: CpuMask::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct SchedEntityWrapper(*mut SchedEntity);

impl SchedEntityWrapper {
    pub fn from_ptr(ptr: *mut SchedEntity) -> Self {
        Self(ptr)
    }
    pub fn sched_entity(&self) -> Option<&SchedEntity> {
        if self.0.is_null() {
            None
        } else {
            unsafe { Some(&*self.0) }
        }
    }
}

impl Default for SchedEntityWrapper {
    fn default() -> Self {
        Self(core::ptr::null_mut())
    }
}

unsafe impl Sync for SchedEntityWrapper {}
unsafe impl Send for SchedEntityWrapper {}

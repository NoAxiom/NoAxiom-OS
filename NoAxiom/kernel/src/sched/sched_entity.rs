use crate::{include::sched::CpuMask, time::time_info::TimeInfo};

pub struct SchedEntity {
    pub time_stat: TimeInfo, // task time
    pub cpu_mask: CpuMask,   // cpu mask
}

impl Default for SchedEntity {
    fn default() -> Self {
        Self {
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
    pub fn sched_entity(&self) -> &SchedEntity {
        unsafe { &*self.0 }
    }
    pub fn sched_entity_mut(&mut self) -> &mut SchedEntity {
        unsafe { &mut *self.0 }
    }
}

impl Default for SchedEntityWrapper {
    fn default() -> Self {
        Self(core::ptr::null_mut())
    }
}

unsafe impl Sync for SchedEntityWrapper {}
unsafe impl Send for SchedEntityWrapper {}

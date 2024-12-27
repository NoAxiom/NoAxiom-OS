//! CFS(completly fair schedule) scheduler entity

use alloc::sync::Arc;
use core::cmp::Ordering;

use crate::{
    constant::sched::{NICE_0_LOAD, SCHED_PRIO_TO_WEIGHT, SCHED_PRIO_TO_WMULT},
    sync::cell::SyncUnsafeCell,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub struct SchedVruntime(pub usize);

impl SchedVruntime {
    pub fn new(vruntime: usize) -> Self {
        Self(vruntime)
    }
    #[inline(always)]
    pub fn update(&mut self, delta: usize) {
        trace!("update vruntime: delta: {}", delta);
        self.0 += delta;
    }
}

impl PartialOrd for SchedVruntime {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let diff = (self.0 - other.0) as i64;
        diff.partial_cmp(&0)
    }
}

/// priority, range: -20 ~ 19
#[derive(Debug)]
pub struct SchedPrio(pub isize);

impl SchedPrio {
    #[inline(always)]
    pub fn to_load_weight(&self) -> usize {
        SCHED_PRIO_TO_WEIGHT[(self.0 + 20) as usize]
    }
    #[inline(always)]
    pub fn to_inv_weight(&self) -> usize {
        SCHED_PRIO_TO_WMULT[(self.0 + 20) as usize]
    }
}

#[derive(Debug)]
pub struct SchedEntityInner {
    /// virtual runtime. scheduler uses this to compare
    pub vruntime: SchedVruntime,

    /// priority, range: -20 ~ 19
    pub prio: SchedPrio,
}

impl SchedEntityInner {
    pub fn new(vruntime: SchedVruntime) -> Self {
        Self {
            vruntime,
            prio: SchedPrio(0),
        }
    }
    /// update vruntime by delta(ms)
    pub fn update_vruntime(&mut self, wall_time: usize) {
        trace!(
            "wall_time: {}, to_inv: {}",
            wall_time,
            self.prio.to_inv_weight()
        );
        self.vruntime
            .update((wall_time * NICE_0_LOAD * self.prio.to_inv_weight()) >> 32);
    }
}

pub struct SchedEntity {
    inner: Arc<SyncUnsafeCell<SchedEntityInner>>,
}

impl SchedEntity {
    pub fn new_bare() -> Self {
        Self {
            inner: Arc::new(SyncUnsafeCell::new(SchedEntityInner::new(
                SchedVruntime::new(0),
            ))),
        }
    }
    #[inline(always)]
    pub fn inner(&self) -> &SchedEntityInner {
        unsafe { &*self.inner.get() }
    }
    #[inline(always)]
    pub fn inner_mut(&self) -> &mut SchedEntityInner {
        unsafe { &mut *self.inner.get() }
    }
    #[inline(always)]
    pub fn update_vruntime(&self, wall_time: usize) {
        self.inner_mut().update_vruntime(wall_time);
    }

    pub fn data_clone(&self) -> Self {
        Self {
            inner: Arc::new(SyncUnsafeCell::new(SchedEntityInner {
                vruntime: SchedVruntime::new(self.inner().vruntime.0),
                prio: SchedPrio(self.inner().prio.0),
            })),
        }
    }
    pub fn ref_clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

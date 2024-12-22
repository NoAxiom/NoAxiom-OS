//! CFS(completly fair schedule) scheduler entity

use core::cmp::Ordering;

use crate::constant::sched::{NICE_0_LOAD, SCHED_PRIO_TO_WEIGHT, SCHED_PRIO_TO_WMULT};

#[derive(Debug, Clone, PartialEq, Eq, Ord)]
pub struct SchedVruntime(u64);

impl SchedVruntime {
    pub fn new(vruntime: u64) -> Self {
        Self(vruntime)
    }
    pub fn update(&mut self, delta: u64) {
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
pub struct SchedPrio(isize);

impl SchedPrio {
    pub fn new(prio: isize) -> Self {
        Self(prio)
    }
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut isize {
        &mut self.0
    }
    #[inline(always)]
    #[allow(unused)]
    pub fn to_weight(&self) -> u64 {
        SCHED_PRIO_TO_WEIGHT[(self.0 + 20) as usize]
    }
    #[inline(always)]
    pub fn to_inv_weight(&self) -> u64 {
        SCHED_PRIO_TO_WMULT[(self.0 + 20) as usize]
    }
}

pub struct SchedEntity {
    /// virtual runtime. scheduler uses this to compare
    pub vruntime: SchedVruntime,

    /// priority, range: -20 ~ 19
    pub prio: SchedPrio,
}

impl SchedEntity {
    pub fn new(vruntime: SchedVruntime) -> Self {
        Self {
            vruntime,
            prio: SchedPrio(0),
        }
    }
    /// update vruntime by delta(ms)
    pub fn update_vruntime(&mut self, wall_time: u64) {
        self.vruntime
            .update((wall_time * NICE_0_LOAD * self.prio.to_inv_weight()) >> 32);
    }
}

use lazy_static::lazy_static;

use super::{
    cfs::CFS,
    multicore::{NoAxiomRuntime, SchedInfo},
};
use crate::sched::vsched::Runtime;

// TODO: add muticore support
lazy_static! {
    pub static ref RUNTIME: NoAxiomRuntime<CFS<SchedInfo>, SchedInfo> = NoAxiomRuntime::new();
}

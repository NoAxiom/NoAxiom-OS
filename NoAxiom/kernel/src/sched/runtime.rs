use lazy_static::lazy_static;

use super::{cfs::CFS, multicore::NoAxiomRuntime, sched_info::SchedInfo, vsched::Runtime};
lazy_static! {
    pub(crate) static ref RUNTIME: NoAxiomRuntime<CFS<SchedInfo>> = NoAxiomRuntime::new();
}

// use super::{
//     simple::{SimpleRuntime, SimpleScheduler},
//     vsched::Runtime,
// };
// lazy_static! {
//     pub static ref RUNTIME: SimpleRuntime<SimpleScheduler> =
// SimpleRuntime::new(); }

pub fn run() {
    RUNTIME.run();
}

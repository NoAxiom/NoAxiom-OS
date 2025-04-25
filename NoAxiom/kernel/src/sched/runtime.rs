use lazy_static::lazy_static;

// use super::cfs::CfsRuntime;
// use crate::sched::vsched::Runtime;
// lazy_static! {
//     pub(crate) static ref RUNTIME: CfsRuntime = CfsRuntime::new();
// }
use super::{
    simple::{SimpleRuntime, SimpleScheduler},
    vsched::Runtime,
};
lazy_static! {
    pub static ref RUNTIME: SimpleRuntime<SimpleScheduler> = SimpleRuntime::new();
}

pub fn run() {
    RUNTIME.run();
}

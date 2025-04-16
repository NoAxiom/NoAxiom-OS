//! # async task schedule
//! ## usages
//! - [`executor`] provides general executor for all types of async tasks
//! - [`scheduler`] provides scheduler for async tasks, usually we use CFS
//! - [`utils`] contains useful func for async tasks execution

mod cfs;
pub mod runtime;
pub mod sched_entity;
pub mod sched_info;
mod simple;
pub mod spawn;
pub mod utils;
pub mod vsched;

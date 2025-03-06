//! # async task schedule
//! ## usages
//! - [`task`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`scheduler`] provides scheduler for async tasks, usually we use CFS
//! - [`utils`] contains useful func for async tasks execution

mod cfs;
pub mod executor;
pub mod sched_entity;
pub mod scheduler;
pub mod spawn;
pub mod utils;

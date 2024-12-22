//! # async task schedule
//! ## usages
//! - [`task`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`utils`] contains useful func for async tasks execution

mod executor;
pub mod sched_entity;
pub mod task;
pub mod task_counter;
pub mod utils;

pub use executor::run;

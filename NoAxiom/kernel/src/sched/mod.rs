//! # async task schedule
//! ## usages
//! - [`utask`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`utils`] contains useful func for async tasks execution

mod executor;
pub mod task_counter;
mod utask;
pub mod utils;

pub use executor::run;
pub use utask::{schedule_spawn_new_process, spawn_task};

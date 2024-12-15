//! # async task schedule
//! ## usages
//! - [`utask`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`utils`] contains useful func for async tasks execution

mod executor;
pub mod ktask;
pub mod task_counter;
pub mod utask;
pub mod utils;

pub use executor::run;
pub use ktask::schedule_spawn_new_ktask;
pub use utask::schedule_spawn_new_process;

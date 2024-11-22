//! # async task schedule
//! ## usages
//! - [`utask`] provides user task behaviour
//! - [`executor`] provides general executor for all types of async tasks
//! - [`utils`] contains useful func for async tasks execution

mod executor;
mod utask;
mod utils;

pub use executor::{run, spawn_raw};
pub use utask::spawn_task;
pub use utils::{take_waker, yield_now};

// schedule test
use crate::println;

pub async fn sched_test() {
    println!("[sched] TEST: async task schedule, cycle 1");
    yield_now().await;
    println!("[sched] TEST: async task schedule, cycle 2");
    yield_now().await;
    println!("[sched] TEST: async task schedule, cycle 3");
    yield_now().await;
    println!("[sched] TEST: async task schedule, done!");
}

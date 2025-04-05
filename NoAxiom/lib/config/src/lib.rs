//! NoAxiom global configs
//! provides constants for kernel config
//! [`cpu`] contains configs for architecture and cpus
//! [`fs`] contains configs for file system
//! [`mm`] contains configs for memory management
//! [`sched`] contains configs for task / coroutine schedule
//! [`task`] contains configs for task / coroutine

#![no_std]

pub mod cpu;
pub mod fs;
pub mod mm;
pub mod sched;
pub mod task;

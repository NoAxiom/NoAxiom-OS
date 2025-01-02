#![no_std]
pub mod cell;
pub mod mutex;

pub use spin::{Lazy, Once};

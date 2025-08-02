#![no_std]
#![allow(deprecated)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

pub mod base_dev;
pub mod basic;
pub mod block;
pub mod char;
pub mod debug;
pub mod display;
pub mod hal;
pub mod interrupt;
mod macros;
pub mod net;
pub mod power;

pub type DevResult<T> = Result<T, include::errno::Errno>;

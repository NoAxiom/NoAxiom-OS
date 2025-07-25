#![no_std]
#![allow(deprecated)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

pub mod base;
pub mod basic;
pub mod block;
pub mod display;
pub mod hal;
pub mod interrupt;
pub(crate) mod macros;
pub mod net;

pub type DevResult<T> = Result<T, include::errno::Errno>;

#![no_std]
#![allow(deprecated)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

pub mod basic;
pub mod hal;
mod impls;
mod macros;
pub mod manager;
mod of;
pub mod probe;

pub use impls::*;

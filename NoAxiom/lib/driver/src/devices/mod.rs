use include::errno::Errno;

pub mod basic;
pub mod block;
pub mod display;
pub mod hal;
pub mod manager;
pub mod net;

pub type DevResult<T> = Result<T, Errno>;

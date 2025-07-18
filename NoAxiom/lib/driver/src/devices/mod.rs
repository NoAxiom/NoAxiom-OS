use include::errno::Errno;

pub mod block;
pub mod gpu;
pub mod hal;
pub mod net;

pub type DevResult<T> = Result<T, Errno>;

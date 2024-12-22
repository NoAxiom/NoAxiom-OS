use core::{fmt::Debug, pin::Pin};

use super::{ADevResult, Device};
use crate::config::errno::Errno;

pub mod uart;

#[macro_export]
macro_rules! here {
    () => {
        concat!(file!(), ":", line!())
    };
}

pub trait CharDevice: Device + Debug {
    fn read<'a>(&'a self, buf: Pin<&'a mut [u8]>) -> ADevResult<isize>;
    fn write(&self, buf: &[u8]) -> Result<(), Errno>;
}

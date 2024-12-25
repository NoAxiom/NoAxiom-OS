use core::{fmt::Debug, pin::Pin};

use crate::utils::result::Errno;

use super::{ADevResult, Device};

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

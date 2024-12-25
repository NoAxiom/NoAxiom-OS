use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

use crate::utils::result::Errno;

pub type BlockReturn<'a> = Pin<Box<dyn Future<Output = Result<isize, Errno>> + Send + 'a>>;

pub trait BlockDevice: Send + Sync {
    fn read<'a>(&'a self, id: usize, buf: &'a mut [u8]) -> BlockReturn;
    fn write<'a>(&'a self, id: usize, buf: &'a [u8]) -> BlockReturn;
    fn flush(&self) -> Result<(), ()>;
    fn close(&self) -> Result<(), ()>;
}

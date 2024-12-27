// pub mod vf2sd;
pub mod virtio;

use super::Driver;
use crate::nix::result::Errno;

pub trait BlockDriver: Driver
where
    Self: Driver,
{
    fn read_block(&self, blk_id: usize, buf: &mut [u8]) -> Result<(), Errno>;
    fn write_block(&self, blk_id: usize, buf: &[u8]) -> Result<(), Errno>;
    fn read_async_block(&self, block_id: usize, buf: &mut [u8]) -> Result<(), Errno>;
    fn write_async_block(&self, block_id: usize, buf: &[u8]) -> Result<(), Errno>;
    fn size(&self) -> usize;
    fn flush(&self) -> Result<(), Errno>;
    fn handle_irq(&self);
}

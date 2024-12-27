use alloc::boxed::Box;

use async_trait::async_trait;

#[async_trait]
pub trait BlockDevice: Send + Sync {
    async fn read<'a>(&'a self, id: usize, buf: &'a mut [u8]);
    async fn write<'a>(&'a self, id: usize, buf: &'a [u8]);
}

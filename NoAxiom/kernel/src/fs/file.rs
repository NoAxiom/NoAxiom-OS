use alloc::{boxed::Box, vec::Vec};
use core::{future::Future, pin::Pin};

use async_trait::async_trait;

#[async_trait]
pub trait File: Send + Sync {
    // ! todo: rename to read
    async fn read_part<'a>(&'a self, offset: usize, len: usize, buf: &'a mut [u8]);
    // ! fixme: delete this temporary function
    fn read<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, ()>> + Send + 'a>>;
    async fn write<'a>(&'a self, buf: &'a [u8]);
}

use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use ext4_rs::BlockDevice;

use crate::fs::vfs::impls::disk_cursor::DiskCursor;

#[async_trait]
impl BlockDevice for DiskCursor {
    async fn read(&self, id: usize) -> Vec<u8> {
        self.base_read_exact_block_size(id).await
    }
    async fn write(&self, id: usize, buf: &[u8]) {
        todo!()
    }
}

use alloc::{boxed::Box, vec::Vec};

use async_trait::async_trait;
use ext4_rs::BlockDevice;

use crate::fs::blockcache::{AsyncBlockCache, CacheData};

#[async_trait]
impl BlockDevice for AsyncBlockCache<CacheData> {
    async fn read(&self, id: usize) -> Vec<u8> {
        self.read_sector(id).await.data.to_vec()
    }
    async fn write(&self, id: usize, buf: &[u8]) {
        assert!(buf.len() == 512);
        let mut fixed_buf = [0u8; 512];
        fixed_buf.copy_from_slice(&buf[..512]);
        self.write_sector(id, &fixed_buf).await
    }
}

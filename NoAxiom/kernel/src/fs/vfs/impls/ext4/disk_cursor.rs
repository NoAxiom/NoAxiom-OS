use alloc::{boxed::Box, sync::Arc, vec::Vec};

use async_trait::async_trait;
use ext4_rs::BlockDevice;
use fatfs::{Read, Seek, Write};
use spin::Mutex;

use crate::fs::vfs::impls::disk_cursor::DiskCursor;

pub struct Ext4DiskCursor {
    blk: Arc<Mutex<DiskCursor>>,
}
impl Ext4DiskCursor {
    pub fn new(blk: Arc<Mutex<DiskCursor>>) -> Self {
        Self { blk }
    }
}

#[async_trait]
impl BlockDevice for Ext4DiskCursor {
    async fn read(&self, id: usize) -> Vec<u8> {
        let mut res = vec![0u8; 4096];
        let mut blk = self.blk.lock();
        let _ = blk.seek(fatfs::SeekFrom::Start(id as u64));
        debug!("read here");
        let _ = self.blk.lock().read_exact(&mut res).await;
        debug!("read here ok");
        res
    }
    async fn write(&self, id: usize, buf: &[u8]) {
        let mut blk = self.blk.lock();
        let _ = blk.seek(fatfs::SeekFrom::Start(id as u64));
        let _ = self.blk.lock().write_all(buf).await;
    }
}

#[async_trait]
impl BlockDevice for DiskCursor {
    async fn read(&self, id: usize) -> Vec<u8> {
        self.base_read_exact_block_size(id).await
    }
    async fn write(&self, id: usize, buf: &[u8]) {
        self.base_write_exact(id, buf).await
    }
}

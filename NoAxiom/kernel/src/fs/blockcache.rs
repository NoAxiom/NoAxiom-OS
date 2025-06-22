//! async block cache

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::num::NonZeroUsize;

use async_trait::async_trait;
use driver::devices::impls::device::{BlockDevice, DevResult};
use ksync::{assert_no_lock, cell::SyncUnsafeCell};
use lru::LruCache;

use crate::config::fs::{BLOCK_SIZE, MAX_LRU_CACHE_SIZE};

lazy_static::lazy_static! {
    pub static ref BLOCK_CACHE: SyncUnsafeCell<AsyncBlockCache> = SyncUnsafeCell::new(AsyncBlockCache {
        cache: SyncUnsafeCell::new(LruCache::new(
            NonZeroUsize::new(MAX_LRU_CACHE_SIZE).unwrap(),
        )),
        block_device: driver::get_blk_dev(),
    });
}

pub fn get_block_cache() -> Arc<&'static dyn BlockDevice> {
    log::debug!("[block_cache] use block cache");
    Arc::new(BLOCK_CACHE.as_ref())
}

/// async block cache with LRU strategy  
pub struct AsyncBlockCache {
    cache: SyncUnsafeCell<LruCache<usize, Arc<[u8; BLOCK_SIZE]>>>, /* todo: use async_mutex, or
                                                                    * doesn't need */
    block_device: Arc<&'static dyn BlockDevice>,
}

// impl Drop for CacheData {
//     fn drop(&mut self) {
//         if self.dirty {
//             info!("CacheData Writeback sector: {}", self.sector);
//         }
//     }
// }

impl AsyncBlockCache {
    /// read a block from the cache or block device  
    /// mind that `sector` == `block`
    pub async fn read_sector(&self, sector: usize) -> Arc<[u8; BLOCK_SIZE]> {
        let cache_guard = self.cache.as_ref_mut();
        if let Some(data) = cache_guard.get(&sector) {
            // if the data is in the cache, return immediately
            return data.clone();
        }

        // else read the data from cache
        let mut data = [0; BLOCK_SIZE];
        assert_no_lock!();
        self.block_device
            .read(sector, &mut data)
            .await
            .expect("read error");
        let data = Arc::new(data);
        let cache_guard = self.cache.as_ref_mut();

        // The key cannot exist in the cache
        let write_back = cache_guard.push(sector, data.clone());
        if let Some((key, value)) = write_back {
            // trace!("read_sector: write back");
            assert_no_lock!();
            let _ = self.block_device.write(key, &*value).await;
        }
        data
    }

    /// write a block to the cache and block device
    /// mind that `sector` == `block`
    pub async fn write_sector(&self, sector: usize, data: &[u8; BLOCK_SIZE]) {
        let cache_guard = self.cache.as_ref_mut();
        let cache_data = cache_guard.get_mut(&sector);
        let data = Arc::new(*data);
        if let Some(cache_data) = cache_data {
            *cache_data = data;
            return;
        }

        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.push(sector, data);
        if let Some((key, value)) = write_back {
            // trace!("write_sector: write back");
            assert_no_lock!();
            let _ = self.block_device.write(key, &*value).await;
        }
    }

    /// flush all dirty data in the cache to the block device
    pub async fn sync_all(&self) {
        trace!("[AsyncBlockCache] cache sync all begin");
        let cache_guard = self.cache.as_ref_mut();
        let mut dirty_data = Vec::new();
        for (sector, cache) in cache_guard.iter() {
            dirty_data.push((*sector, cache.clone()));
        }
        for (sector, data) in dirty_data {
            assert_no_lock!();
            let _ = self.block_device.write(sector, &*data).await;
        }
        info!("[AsyncBlockCache] cache sync all!");
    }
}

#[async_trait]
impl BlockDevice for AsyncBlockCache {
    fn device_name(&self) -> &'static str {
        "AsyncBlockCache"
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        assert_eq!(buf.len(), BLOCK_SIZE);
        let data = self.read_sector(id).await;
        buf.copy_from_slice(&*data);
        Ok(buf.len())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        assert_eq!(buf.len(), BLOCK_SIZE);
        let mut data: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
        data.copy_from_slice(buf);
        self.write_sector(id, &data).await;
        Ok(buf.len())
    }
    async fn sync_all(&self) -> DevResult<()> {
        self.sync_all().await;
        Ok(())
    }
}

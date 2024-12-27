//! async block cache
//!
//! not sure the RwLock

use alloc::sync::Arc;
use core::num::NonZeroUsize;

use lru::LruCache;
use spin::{Mutex, RwLock};

use crate::{
    config::fs::{BLOCK_SIZE, MAX_LRU_CACHE_SIZE},
    device::block::BlockDevice,
};

/// async block cache for data struct `B` with LRU strategy  
/// for either **one writer** or many readers
pub struct AsyncBlockCache<B> {
    cache: Mutex<LruCache<usize, Arc<RwLock<B>>>>, // todo: async_mutex ?
    block_device: Arc<dyn BlockDevice>,
}

pub struct CacheData {
    pub data: [u8; BLOCK_SIZE],
    pub dirty: bool,
}

impl CacheData {
    fn from(data: [u8; BLOCK_SIZE], dirty: bool) -> Self {
        Self { data, dirty }
    }
}

impl AsyncBlockCache<CacheData> {
    /// create a new `AsyncBlockCache` and clear the cache
    pub fn from(device: Arc<dyn BlockDevice>) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(MAX_LRU_CACHE_SIZE).unwrap(),
            )),
            block_device: device,
        }
    }

    /// read a block from the cache or block device  
    /// mind that `sector` == `block`
    pub async fn read_sector(&self, sector: usize) -> Arc<RwLock<CacheData>> {
        let mut cache_guard = self.cache.lock();
        if let Some(data) = cache_guard.get(&sector) {
            // if the data is in the cache, return immediately
            return data.clone();
        }

        // else read the data from cache
        let mut data = [0; BLOCK_SIZE]; // todo: use vector
        let _ = self.block_device.read(sector, &mut data).await;
        let res = Arc::new(RwLock::new(CacheData::from(data, false)));

        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.put(sector, res.clone());
        if let Some(old_data) = write_back {
            let _ = self
                .block_device
                .write(sector, old_data.read().data.as_ref())
                .await;
        }

        res
    }

    /// write a block to the cache and block device
    /// mind that `sector` == `block`
    pub async fn write_sector(&self, sector: usize, data: &[u8; BLOCK_SIZE]) {
        let mut cache_guard = self.cache.lock();
        let res = Arc::new(RwLock::new(CacheData::from(*data, true)));

        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.put(sector, res.clone());
        if let Some(old_data) = write_back {
            let _ = self
                .block_device
                .write(sector, old_data.read().data.as_ref())
                .await;
        }
    }

    /// flush all dirty data in the cache to the block device
    pub async fn sync_all(&self) {
        let cache_guard = self.cache.lock();
        for (sector, data) in cache_guard.iter() {
            if data.read().dirty {
                let _ = self
                    .block_device
                    .write(*sector, data.read().data.as_ref())
                    .await;
            }
        }
    }
}

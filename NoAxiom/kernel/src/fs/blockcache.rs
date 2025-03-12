//! async block cache
//!
//! not sure the RwLock

use alloc::{sync::Arc, vec::Vec};
use core::num::NonZeroUsize;

use ksync::mutex::check_no_lock;
use lru::LruCache;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use crate::{
    config::fs::{BLOCK_SIZE, MAX_LRU_CACHE_SIZE},
    device::block::BlockDevice,
};

/// async block cache for data struct `B` with LRU strategy  
pub struct AsyncBlockCache<B> {
    cache: Mutex<LruCache<usize, B>>, // todo: use async_mutex
    block_device: Arc<dyn BlockDevice>,
}

#[derive(Clone)]
pub struct CacheData {
    pub data: Arc<[u8; BLOCK_SIZE]>,
    pub dirty: bool,
}

impl CacheData {
    fn from(data: [u8; BLOCK_SIZE], dirty: bool) -> Self {
        Self {
            data: Arc::new(data),
            dirty,
        }
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
    pub async fn read_sector(&self, sector: usize) -> CacheData {
        let mut cache_guard = self.cache.lock();
        if let Some(data) = cache_guard.get(&sector) {
            // if the data is in the cache, return immediately
            return data.clone();
        }
        drop(cache_guard);

        // else read the data from cache
        let mut data = [0; BLOCK_SIZE];
        assert!(check_no_lock());
        self.block_device.read(sector, &mut data).await;
        let res = CacheData::from(data, false);

        let mut cache_guard = self.cache.lock();
        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.put(sector, res.clone());
        drop(cache_guard);
        if let Some(old_data) = write_back {
            debug!("write back");
            assert!(check_no_lock());
            let _ = self.block_device.write(sector, &(*old_data.data)).await;
        }

        res
    }

    /// write a block to the cache and block device
    /// mind that `sector` == `block`
    pub async fn write_sector(&self, sector: usize, data: &[u8; BLOCK_SIZE]) {
        let mut cache_guard = self.cache.lock();
        let res = CacheData::from(*data, true);

        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.put(sector, res.clone());
        drop(cache_guard);
        if let Some(old_data) = write_back {
            assert!(check_no_lock());
            let _ = self.block_device.write(sector, &(*old_data.data)).await;
        }
    }

    /// flush all dirty data in the cache to the block device
    pub async fn sync_all(&self) {
        let cache_guard = self.cache.lock();
        let mut dirty_data = Vec::new();
        for (sector, cache) in cache_guard.iter() {
            if cache.dirty {
                dirty_data.push((*sector, cache.clone()));
            }
        }
        drop(cache_guard);
        for (sector, cache) in dirty_data {
            assert!(check_no_lock());
            let _ = self.block_device.write(sector, &(*cache.data)).await;
        }
    }
}

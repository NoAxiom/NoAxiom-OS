//! async block cache
//!
//! not sure the RwLock

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::num::NonZeroUsize;

use async_trait::async_trait;
use ksync::mutex::check_no_lock;
use lru::LruCache;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use crate::{
    config::fs::{BLOCK_SIZE, MAX_LRU_CACHE_SIZE},
    device::block::BlockDevice,
};

/// async block cache for data struct `B` with LRU strategy  
pub struct AsyncBlockCache<B: Cache + Clone> {
    cache: Mutex<LruCache<usize, B>>, // todo: use async_mutex
    block_device: Arc<dyn BlockDevice>,
}

pub trait Cache {
    fn dirty(&self) -> bool;
    fn data(&self) -> &[u8; BLOCK_SIZE];
    fn from(data: [u8; BLOCK_SIZE], dirty: bool) -> Self;
}

#[derive(Clone)]
pub struct CacheData {
    pub data: Arc<[u8; BLOCK_SIZE]>,
    pub dirty: bool,
}

impl Cache for CacheData {
    fn dirty(&self) -> bool {
        self.dirty
    }
    fn data(&self) -> &[u8; BLOCK_SIZE] {
        &*self.data
    }
    fn from(data: [u8; BLOCK_SIZE], dirty: bool) -> Self {
        Self {
            data: Arc::new(data),
            dirty,
        }
    }
}

// impl Drop for CacheData {
//     fn drop(&mut self) {
//         if self.dirty {
//             info!("CacheData Writeback sector: {}", self.sector);
//         }
//     }
// }

impl<B: Cache + Clone> AsyncBlockCache<B> {
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
    pub async fn read_sector(&self, sector: usize) -> B {
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
        let res = B::from(data, false);

        let mut cache_guard = self.cache.lock();

        // The key cannot exist in the cache
        let write_back = cache_guard.push(sector, res.clone());
        drop(cache_guard);
        if let Some((key, value)) = write_back {
            // trace!("read_sector: write back");
            assert!(check_no_lock());
            let _ = self.block_device.write(key, value.data()).await;
        }
        res
    }

    /// write a block to the cache and block device
    /// mind that `sector` == `block`
    pub async fn write_sector(&self, sector: usize, data: &[u8; BLOCK_SIZE]) {
        let mut cache_guard = self.cache.lock();
        let cache_data = cache_guard.get_mut(&sector);
        let res = B::from(*data, true);

        if let Some(cache_data) = cache_data {
            *cache_data = res;
            return;
        }

        // If the key already exists in the cache, write back the old data
        let write_back = cache_guard.push(sector, res);
        drop(cache_guard);
        if let Some((key, value)) = write_back {
            // trace!("write_sector: write back");
            assert!(check_no_lock());
            let _ = self.block_device.write(key, value.data()).await;
        }
    }

    /// flush all dirty data in the cache to the block device
    pub async fn sync_all(&self) {
        trace!("[AsyncBlockCache] cache sync all begin");
        let cache_guard = self.cache.lock();
        let mut dirty_data = Vec::new();
        for (sector, cache) in cache_guard.iter() {
            if cache.dirty() {
                dirty_data.push((*sector, cache.clone()));
            }
        }
        drop(cache_guard);
        for (sector, cache) in dirty_data {
            assert!(check_no_lock());
            let _ = self.block_device.write(sector, cache.data()).await;
        }
        info!("[AsyncBlockCache] cache sync all!");
    }
}

#[async_trait]
impl BlockDevice for AsyncBlockCache<CacheData> {
    async fn read(&self, _id: usize, _buf: &mut [u8]) {
        unreachable!()
    }

    async fn write(&self, _id: usize, _buf: &[u8]) {
        unreachable!()
    }

    async fn sync_all(&self) {
        self.sync_all().await
    }
}

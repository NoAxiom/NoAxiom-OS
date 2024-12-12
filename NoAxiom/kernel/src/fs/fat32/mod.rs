//! reference: https://d1.amobbs.com/bbs_upload782111/files_7/armok01151038.pdf

mod bpb;
mod directory;
mod entry;
mod fat;

use alloc::sync::Arc;

use directory::RootDirectory;

use super::{
    blockcache::{AsyncBlockCache, CacheData},
    blockdevice::BlockDevice,
};
use crate::config::fs::{FAT32_SECTOR_SIZE, FIRST_CLUSTER};

type ABC = AsyncBlockCache<CacheData>;

pub struct FAT32FIleSystem {
    blk: Arc<ABC>,
    bpb: [u8; FAT32_SECTOR_SIZE],
    fat: fat::FAT,
    // todo: tree: NTree<String, Vec<u8>, Vec<u32>>,
    root: RootDirectory,
}

impl FAT32FIleSystem {
    pub async fn init(device: Arc<dyn BlockDevice>) -> Self {
        let bpb = {
            let mut sector = [0u8; FAT32_SECTOR_SIZE]; // todo: use vec
            let _ = device.read(0, &mut sector).await;
            sector
        };
        let blk = Arc::new(AsyncBlockCache::from(device));
        let fat = fat::FAT::new(&bpb);
        let root = RootDirectory::new(
            bpb.clone(),
            fat.get_link(&blk, FIRST_CLUSTER).await,
            Arc::clone(&blk),
        );

        Self {
            blk,
            bpb,
            fat,
            root,
        }
    }
}

//! reference: https://d1.amobbs.com/bbs_upload782111/files_7/armok01151038.pdf

// ignore warnings for this module
#![allow(warnings)]

mod bpb;
pub mod directory;
mod entry;
mod fat;
pub mod file;

use alloc::{string::String, sync::Arc};

use bpb::BIOSParameterBlockOffset;
use directory::{FAT32Directory, ShortDirectory};
use downcast_rs::{impl_downcast, DowncastSync};
use entry::ShortDirectoryEntry;

use super::blockcache::{AsyncBlockCache, CacheData};
use crate::{
    config::fs::{FAT32_SECTOR_SIZE, FIRST_CLUSTER, ROOT_FAKE_ENTRY},
    device::block::BlockDevice,
    nix::fs::InodeMode,
};

pub trait DirFile: Send + Sync + DowncastSync {
    fn name(&self) -> String;
    fn file_type(&self) -> InodeMode;
}
impl_downcast!(sync DirFile);

type ABC = AsyncBlockCache<CacheData>;

pub struct FAT32FIleSystem {
    blk: Arc<ABC>,
    bpb: [u8; FAT32_SECTOR_SIZE],
    fat: Arc<fat::FAT>,
    /// File ident: string, File content: Vec<u8>
    root: String,
}

impl FAT32FIleSystem {
    /// Load bpb and root cluster to get root entry content
    pub async fn load_root(device: Arc<dyn BlockDevice>) -> FAT32Directory {
        #[cfg(feature = "async_fs")]
        {
            use arch::interrupt::{is_external_interrupt_enabled, is_interrupt_enabled};
            assert!(is_interrupt_enabled());
            assert!(is_external_interrupt_enabled());
        }

        let mut bpb = [0u8; FAT32_SECTOR_SIZE];
        device.read(0, &mut bpb).await;

        let bpb = bpb;
        // normally, root cluster is 2
        let root_cluster = BIOSParameterBlockOffset::root_cluster(&bpb);
        assert_eq!(root_cluster, FIRST_CLUSTER, "bpb: {:?}", bpb);

        let blk = Arc::new(AsyncBlockCache::from(device));
        let fat = Arc::new(fat::FAT::new(&bpb));
        let bpb = Arc::new(bpb);

        // ! fixme: Now load all the content into memory as cache, avoid read disk
        // ! later. Because read/write disk later should turn on the interrupt, which is
        // ! dangerous when the hart be sched.
        // for block_id in 0..1000 {
        //     blk.read_sector(block_id).await;
        // }
        // blk.read_sector(23).await;
        // blk.read_sector(23).await;
        // blk.read_sector(23).await;

        // check the ROOT_FAKE_ENTRY
        let root_entry = ShortDirectoryEntry::from(ROOT_FAKE_ENTRY);
        assert_eq!(root_entry.first_cluster(), root_cluster);

        // get root entry
        let root = ShortDirectory::from(
            root_entry,
            Arc::clone(&fat),
            Arc::clone(&bpb),
            Arc::clone(&blk),
        );

        FAT32Directory::from_short(root)
    }
    /// Get empty file system binding with `device`
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        let blk = Arc::new(AsyncBlockCache::from(device));
        let bpb = [0u8; FAT32_SECTOR_SIZE];
        let fat = Arc::new(fat::FAT::new(&bpb));
        let root = String::from("/");
        Self {
            blk,
            bpb,
            fat,
            root,
        }
    }
}

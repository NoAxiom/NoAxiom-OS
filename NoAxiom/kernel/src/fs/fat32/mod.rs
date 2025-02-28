//! reference: https://d1.amobbs.com/bbs_upload782111/files_7/armok01151038.pdf

// ignore warnings for this module
// #![allow(warnings)]

mod bpb;
pub mod directory;
mod entry;
mod fat;
pub mod file;
mod tree;

use alloc::{string::String, sync::Arc, vec::Vec};

use arch::{Arch, ArchInt};
use bpb::{cluster_offset_sectors, BIOSParameterBlockOffset};
use directory::{FAT32Directory, ShortDirectory};
use downcast_rs::{impl_downcast, DowncastSync};
use entry::ShortDirectoryEntry;
use tree::NTree;

use super::blockcache::{AsyncBlockCache, CacheData};
use crate::{
    config::fs::{FAT32_SECTOR_SIZE, FIRST_CLUSTER, ROOT_FAKE_ENTRY},
    device::block::BlockDevice,
    include::fs::InodeMode,
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
    tree: Option<NTree<String, Vec<u8>, Vec<u32>>>,
}

impl FAT32FIleSystem {
    /// Load bpb and root cluster to get root entry content
    pub async fn load_root(device: Arc<dyn BlockDevice>) -> FAT32Directory {
        #[cfg(feature = "async_fs")]
        {
            assert!(Arch::is_interrupt_enabled());
            assert!(Arch::is_external_interrupt_enabled());
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
            tree: None,
        }
    }

    // fn is_long<S: AsRef<str>>(s: &S) -> bool {
    //     let s = s.as_ref();
    //     match s.len() {
    //         0 => panic!("empty name!"),
    //         1..=11 => match s.contains(".") {
    //             true => {
    //                 let mut s = s.split(".").collect::<Vec<_>>();
    //                 let ext = s.pop().unwrap();
    //                 if ext.len() > 3 {
    //                     true
    //                 } else {
    //                     let len = s.iter().fold(0, |acc, &x| acc + x.len());
    //                     len > 8
    //                 }
    //             }
    //             false => s.len() > 8,
    //         },
    //         _ => true,
    //     }
    // }

    // pub async fn create(&mut self, dir: String, file: String, size: u32) ->
    // Result<(), ()> {     let node =
    // self.tree.as_mut().unwrap().find_mut(dir);     if node.is_none() {
    //         return Err(());
    //     }
    //     let s: String = file.into();
    //     match Self::is_long(&s) {
    //         false => {
    //             // 短文件名
    //             let mut name = [0x20; 8];
    //             let mut extension = [0x20; 3];
    //             match s.contains(".") {
    //                 true => {
    //                     let mut v = s.split(".").collect::<Vec<_>>();
    //                     let last = v.pop().unwrap();
    //                     extension.copy_from_slice(last.as_bytes());
    //                     for (idx, c) in v.iter().flat_map(|ss|
    // ss.chars()).enumerate() {                         name[idx] = c as u8;
    //                     }
    //                 }
    //                 false => name[0..s.len()].copy_from_slice(s.as_bytes()),
    //             }
    //             if let Some(fst_cluster) =
    // self.fat.find_free_cluster_id(&self.blk).await {                 // 标记
    // `fat` 表为已占用                 self.fat
    //                     .set_cluster_id(&*self.blk, fst_cluster, 0xfffffff)
    //                     .await;
    //                 let mut last = fst_cluster;
    //                 // 分配足够的 `FAT` 表项
    //                 for _ in 0..size - 1 {
    //                     let new_cluster = self
    //                         .fat
    //                         .find_free_cluster_id(&self.blk)
    //                         .await
    //                         .map_or_else(|| panic!("no avaiable space!"), |x| x);
    //                     self.fat.set_cluster_id(&*self.blk, last,
    // new_cluster).await;                     last = new_cluster;
    //                 }
    //                 // 更新最后一项 `FAT` 表
    //                 self.fat.set_cluster_id(&self.blk, last, 0xfffffff).await;
    //                 let first_cluster_low = fst_cluster as u16;
    //                 let first_cluster_high = (fst_cluster >> 16) as u16;
    //                 let entry = ShortDirectoryEntry {
    //                     name,
    //                     extension,
    //                     attribute: Attribute::ATTR_ARCHIVE,
    //                     _reserved: 0,
    //                     file_size: size * BLOCK_SIZE as u32,
    //                     first_cluster_low,
    //                     first_cluster_high,
    //                     ..Default::default()
    //                 };
    //                 let node = node.unwrap();
    //                 // 下面将 entry 写入块设备
    //                 // 获取父节点结点占用的块号
    //                 let clusters = node.inner().content_ref().await;
    //                 let mut has_free = false;
    //                 let mut free_entry = (0, 0);
    //                 for cluster in &clusters {
    //                     // 获取块号对应的扇区偏移
    //                     let sector = cluster_offset_sectors(&self.bpb, *cluster);
    //                     let block = self.blk.read_sector(sector as usize).await;
    //                     let block = block.read().data;
    //                     for (idx, fat) in block.chunks(32).enumerate() {
    //                         if fat.iter().all(|b| *b == 0x0) {
    //                             has_free = true;
    //                             free_entry = (sector, idx);
    //                             break;
    //                         }
    //                     }
    //                     if has_free {
    //                         break;
    //                     }
    //                 }
    //                 if has_free {
    //                     // 如果有空的 `FAT` 表项
    //                     let block = self.blk.read_sector(free_entry.0 as
    // usize).await;                     let mut block = block.read().data;
    //                     for (idx, e) in block.chunks_mut(32).enumerate() {
    //                         if idx == free_entry.1 {
    //                             let new_e: [u8; 32] = entry.clone().as_slice();
    //                             e.copy_from_slice(&new_e);
    //                             break;
    //                         }
    //                     }
    //                     // 写回块设备
    //                     self.blk.write_sector(free_entry.0 as usize,
    // &block).await;                 } else {
    //                     //
    // 如果父节点占据的块里面所有目录项都被占用了，则需要申请新的块
    // if let Some(new_cluster) = self.fat.find_free_cluster_id(&self.blk).await {
    //                         // 父节点最后的块号
    //                         let last = *clusters.last().unwrap();
    //                         // 更新 `FAT` 表
    //                         self.fat.set_cluster_id(&self.blk, last,
    // new_cluster).await;                         self.fat
    //                             .set_cluster_id(&self.blk, new_cluster,
    // 0xfffffff)                             .await;
    //                         // 将新的块读取进内存
    //                         let sector = cluster_offset_sectors(&self.bpb,
    // new_cluster);                         let block =
    // self.blk.read_sector(sector as usize).await;                         let
    // mut block = block.read().data;                         // 设置第一项的值
    //                         let e: [u8; 32] = entry.clone().as_slice();
    //                         block[0..32].copy_from_slice(&e);
    //                         // 写回块设备
    //                         self.blk.write_sector(sector as usize, &block).await;
    //                     } else {
    //                         panic!("no avaiable space in disk!")
    //                     }
    //                 }
    //                 let file = ShortFile::from(
    //                     entry,
    //                     Arc::clone(&self.fat),
    //                     Arc::new(self.bpb.clone()),
    //                     Arc::clone(&self.blk),
    //                 );
    //                 // 更新目录树
    //                 node.insert(Box::new(file));
    //                 Ok(())
    //             } else {
    //                 panic!("no avaiable space in disk!")
    //             }
    //         }
    //         true => {
    //             // 长文件名
    //             todo!()
    //         }
    //     }
    // }
}

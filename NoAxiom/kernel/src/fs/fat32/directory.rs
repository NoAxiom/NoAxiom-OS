use alloc::{string::String, sync::Arc, vec::Vec};

use super::{bpb::cluster_offset_sectors, entry::ShortDirectoryEntry, fat::FAT, ABC};
use crate::config::fs::BLOCK_SIZE;

pub struct ShortDirectory {
    entry: ShortDirectoryEntry,
    bpb: Arc<[u8; BLOCK_SIZE]>,
    fat: Arc<FAT>,
    blk: Arc<ABC>,
}

impl ShortDirectory {
    pub fn new(
        entry: ShortDirectoryEntry,
        fat: Arc<FAT>,
        bpb: Arc<[u8; BLOCK_SIZE]>,
        blk: Arc<ABC>,
    ) -> Self {
        Self {
            entry,
            bpb,
            fat,
            blk,
        }
    }
    pub fn name(&self) -> String {
        self.entry.name()
    }
    pub async fn data(&self) -> Vec<u8> {
        self.entry.load(&self.blk, &self.fat, &self.bpb).await
    }
    /// get entries
    pub async fn children(&self) -> Vec<[u8; 32]> {
        let data = self.data().await;
        data.chunks(32)
            .map(|b| {
                let mut entry = [0; 32];
                entry.copy_from_slice(b);
                entry
            })
            .collect()
    }
}

// todo: LongDirectory

#[derive(Clone)]
pub struct RootDirectory {
    name: String,
    bpb: Arc<[u8; BLOCK_SIZE]>,
    clusters: Vec<u32>,
    blk: Arc<ABC>,
}

impl RootDirectory {
    pub fn new(bpb: [u8; BLOCK_SIZE], clusters: Vec<u32>, blk: Arc<ABC>) -> Self {
        Self {
            name: String::from("/"),
            bpb: Arc::new(bpb),
            clusters,
            blk,
        }
    }
    fn identify(&self, ident: &String) -> bool {
        &self.name == ident
    }
    fn ident(&self) -> String {
        self.name.clone()
    }

    /// get content
    async fn content(&self) -> Vec<u8> {
        let mut ret = Vec::new();
        for cluster in &self.clusters {
            let cluster = cluster_offset_sectors(&*self.bpb, *cluster);
            let sector = self.blk.read_sector(cluster as usize).await;
            sector.read().data.iter().for_each(|b| ret.push(*b));
        }
        ret
    }
    fn content_ref(&self) -> Vec<u32> {
        self.clusters.clone()
    }
}

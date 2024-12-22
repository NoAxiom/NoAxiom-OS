use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use async_trait::async_trait;

use super::{
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    fs_node::FSNode,
};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::fat32::{bpb::cluster_offset_sectors, fat::FAT, ABC},
};

#[derive(Clone)]
/// the short directory type in the file tree
pub struct ShortDirectory {
    entry: ShortDirectoryEntry,
    bpb: Arc<[u8; BLOCK_SIZE]>,
    fat: Arc<FAT>,
    blk: Arc<ABC>,
}

impl ShortDirectory {
    pub fn from(
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
    /// get entries
    pub async fn children(&self) -> Vec<[u8; 32]> {
        let data = self.content().await;
        data.chunks(32)
            .map(|x| {
                let mut entry = [0; 32];
                entry.copy_from_slice(x);
                entry
            })
            .collect()
    }
}

#[async_trait]
impl FSNode<String, Vec<u8>> for ShortDirectory {
    async fn content(&self) -> Vec<u8> {
        self.entry.load(&self.blk, &self.fat, &self.bpb).await
    }

    fn ident(&self) -> String {
        self.entry.name()
    }
}

#[derive(Clone)]
/// the long directory type in the file tree
pub struct LongDirectory {
    short_dir: ShortDirectory,
    long_entries: Vec<LongDirectoryEntry>,
}

impl LongDirectory {
    pub fn from(
        entry: ShortDirectoryEntry,
        fat: Arc<FAT>,
        bpb: Arc<[u8; BLOCK_SIZE]>,
        blk: Arc<ABC>,
        long_entries: Vec<LongDirectoryEntry>,
    ) -> Self {
        Self {
            short_dir: ShortDirectory::from(entry, fat, bpb, blk),
            long_entries,
        }
    }
    pub async fn children(&self) -> Vec<[u8; 32]> {
        self.short_dir.children().await
    }
}

#[async_trait]
impl FSNode<String, Vec<u8>> for LongDirectory {
    async fn content(&self) -> Vec<u8> {
        self.short_dir.content().await
    }

    fn ident(&self) -> String {
        let mut name = String::new();
        for l in &self.long_entries {
            l.name().iter().for_each(|c| name.push(*c));
        }
        name
    }
}

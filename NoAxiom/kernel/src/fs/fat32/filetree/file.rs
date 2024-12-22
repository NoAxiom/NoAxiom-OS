use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

use async_trait::async_trait;

use super::{
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    fs_node::FSNode,
};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::fat32::{fat::FAT, ABC},
};

/// the short file type in the file tree
pub struct ShortFile {
    entry: ShortDirectoryEntry,
    bpb: Arc<[u8; BLOCK_SIZE]>,
    fat: Arc<FAT>,
    blk: Arc<ABC>,
}

impl ShortFile {
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
    pub fn size(&self) -> u32 {
        self.entry.file_size
    }
}

#[async_trait]
impl FSNode<String, Vec<u8>> for ShortFile {
    async fn content(&self) -> Vec<u8> {
        let content = self.entry.load(&self.blk, &self.fat, &self.bpb).await;
        content[0..self.size() as usize].to_vec()
    }

    fn ident(&self) -> String {
        self.entry.name()
    }
}

/// the long file type in the file tree, but is equal to the short file?
pub struct LongFile {
    short_file: ShortFile,
    long_entries: Vec<LongDirectoryEntry>,
}

impl LongFile {
    pub fn from(
        entry: ShortDirectoryEntry,
        fat: Arc<FAT>,
        bpb: Arc<[u8; BLOCK_SIZE]>,
        blk: Arc<ABC>,
        long_entries: Vec<LongDirectoryEntry>,
    ) -> Self {
        Self {
            short_file: ShortFile::from(entry, fat, bpb, blk),
            long_entries,
        }
    }
    pub fn size(&self) -> u32 {
        self.short_file.size()
    }
}

#[async_trait]
impl FSNode<String, Vec<u8>> for LongFile {
    async fn content(&self) -> Vec<u8> {
        self.short_file.content().await
    }

    fn ident(&self) -> String {
        let mut name = String::new();
        for l in &self.long_entries {
            l.name().iter().for_each(|c| name.push(*c));
        }
        name
    }
}

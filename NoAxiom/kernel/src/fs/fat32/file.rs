use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

use async_trait::async_trait;

use super::{
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    tree::AsNode,
    DirFile,
};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::fat32::{fat::FAT, ABC},
    syscall::SyscallResult,
};

/// the short file type in the file tree
#[derive(Clone)]
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

impl ShortFile {
    pub async fn content(&self) -> Vec<u8> {
        self.entry.load(&self.blk, &self.fat, &self.bpb).await
    }
    pub async fn part_content<'a>(&'a self, offset: usize, buf: &'a mut [u8]) -> SyscallResult {
        let len = buf.len();
        let content = self.entry.load(&self.blk, &self.fat, &self.bpb).await;
        buf.copy_from_slice(&content[offset..offset + len]);
        Ok(len as isize)
    }
    pub async fn store_part_content<'a>(&'a self, offset: usize, buf: &'a [u8]) -> SyscallResult {
        let len = buf.len();
        self.entry
            .store(&self.blk, &self.fat, &self.bpb, offset, buf)
            .await;
        Ok(len as isize)
    }
    pub fn ident(&self) -> String {
        self.entry.name()
    }
}

#[async_trait]
impl AsNode for ShortFile {
    type Ident = String;
    type Content = Vec<u8>;
    type ContentRef = Vec<u32>;
    fn identify(&self, ident: &Self::Ident) -> bool {
        self.ident() == *ident
    }
    fn ident(&self) -> Self::Ident {
        self.ident()
    }
    async fn content(&self) -> Self::Content {
        let ret = self.content().await;
        ret[..self.size() as usize].to_vec()
    }
    async fn content_ref(&self) -> Self::ContentRef {
        self.entry.clusters(&self.blk, &self.fat).await
    }
}

/// the long file type in the file tree, but is equal to the short file?
#[derive(Clone)]
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
}

impl LongFile {
    pub fn ident(&self) -> String {
        let mut name = String::new();
        for l in &self.long_entries {
            l.name().iter().for_each(|c| name.push(*c));
        }
        name
    }
}

#[async_trait]
impl AsNode for LongFile {
    type Ident = String;
    type Content = Vec<u8>;
    type ContentRef = Vec<u32>;
    fn identify(&self, ident: &Self::Ident) -> bool {
        self.ident() == *ident
    }
    fn ident(&self) -> Self::Ident {
        self.ident()
    }
    async fn content(&self) -> Self::Content {
        let ret = self.short_file.content().await;
        ret[..self.short_file.size() as usize].to_vec()
    }
    async fn content_ref(&self) -> Self::ContentRef {
        self.short_file
            .entry
            .clusters(&self.short_file.blk, &self.short_file.fat)
            .await
    }
}

// todo: use better implementation
#[derive(Clone)]
pub struct FAT32File {
    inner: LongFile,
}

impl DirFile for FAT32File {
    fn name(&self) -> String {
        self.ident()
    }
    fn file_type(&self) -> crate::nix::fs::InodeMode {
        crate::nix::fs::InodeMode::FILE
    }
}

impl FAT32File {
    pub fn from_long(inner: LongFile) -> Self {
        Self { inner }
    }
    pub fn from_short(inner: ShortFile) -> Self {
        Self {
            inner: LongFile::from(inner.entry, inner.fat, inner.bpb, inner.blk, Vec::new()),
        }
    }
    fn ident(&self) -> String {
        if self.inner.long_entries.is_empty() {
            self.inner.short_file.ident()
        } else {
            self.inner.ident()
        }
    }
    pub fn size(&self) -> usize {
        self.inner.short_file.size() as usize
    }
}

impl FAT32File {
    pub async fn read_from(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        self.inner.short_file.part_content(offset, buf).await
    }
    pub async fn write_at(&self, offset: usize, buf: &[u8]) -> SyscallResult {
        self.inner.short_file.store_part_content(offset, buf).await
    }
}

use alloc::{string::String, sync::Arc, vec::Vec};

use super::{
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    DirFile,
};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::fat32::{
        fat::FAT,
        file::{FAT32File, LongFile, ShortFile},
        ABC,
    },
    utils::reverse,
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
    async fn content(&self) -> Vec<u8> {
        self.entry.load(&self.blk, &self.fat, &self.bpb).await
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
    fn ident(&self) -> String {
        let mut name = String::new();
        for l in &self.long_entries {
            l.name().iter().for_each(|c| name.push(*c));
        }
        name
    }
}

// todo: use better implementation
#[derive(Clone)]
pub struct FAT32Directory {
    inner: LongDirectory,
}

impl DirFile for FAT32Directory {
    fn name(&self) -> String {
        self.inner.ident()
    }
    fn file_type(&self) -> crate::nix::fs::InodeMode {
        crate::nix::fs::InodeMode::DIR
    }
}

impl FAT32Directory {
    pub fn from_long(inner: LongDirectory) -> Self {
        Self { inner }
    }
    pub fn from_short(inner: ShortDirectory) -> Self {
        Self {
            inner: LongDirectory::from(inner.entry, inner.fat, inner.bpb, inner.blk, Vec::new()),
        }
    }
}

impl FAT32Directory {
    fn ident(&self) -> String {
        if self.inner.long_entries.is_empty() {
            self.inner.short_dir.ident()
        } else {
            self.inner.ident()
        }
    }
    pub async fn load(&self) -> Vec<Arc<dyn DirFile>> {
        let mut res: Vec<Arc<dyn DirFile>> = Vec::new();

        let fat = self.inner.short_dir.fat.clone();
        let bpb = self.inner.short_dir.bpb.clone();
        let blk = self.inner.short_dir.blk.clone();

        let mut long = false;
        let mut long_entries = Vec::new();
        let content = self.inner.short_dir.content().await;
        for entry in content.as_slice().chunks(32) {
            let mut e = [0; 32];
            e.copy_from_slice(entry);
            match entry[11] {
                0x10 => {
                    let e = ShortDirectoryEntry::from(e);
                    if e.is_dot() || e.is_dotdot() || e.is_free() || e.is_deleted() {
                        if long {
                            long_entries.clear();
                            long = false;
                        }
                        continue;
                    }
                    match long {
                        true => {
                            // long file entry
                            long = false;
                            let long_dir = LongDirectory::from(
                                e,
                                Arc::clone(&fat),
                                Arc::clone(&bpb),
                                Arc::clone(&blk),
                                reverse(&long_entries),
                            );
                            long_entries.clear();
                            debug!("insert long dir: {}", long_dir.ident());
                            res.push(Arc::from(FAT32Directory::from_long(long_dir)));
                        }
                        false => {
                            // short file entry
                            let short_dir = ShortDirectory::from(
                                e,
                                Arc::clone(&fat),
                                Arc::clone(&bpb),
                                Arc::clone(&blk),
                            );
                            debug!("insert short dir: {}", short_dir.ident());
                            res.push(Arc::from(FAT32Directory::from_short(short_dir)));
                        }
                    }
                }
                0x0f => {
                    // long file entry
                    long = true;
                    long_entries.push(LongDirectoryEntry::from(e));
                }
                0x01 | 0x02 | 0x04 | 0x08 | 0x20 => {
                    let e = ShortDirectoryEntry::from(e);
                    if e.is_dot() || e.is_dotdot() || e.is_free() || e.is_deleted() {
                        if long {
                            long_entries.clear();
                            long = false;
                        }
                        continue;
                    }
                    match long {
                        true => {
                            // long file entry
                            long = false;
                            let long_file = LongFile::from(
                                e,
                                Arc::clone(&fat),
                                Arc::clone(&bpb),
                                Arc::clone(&blk),
                                reverse(&long_entries),
                            );
                            long_entries.clear();
                            debug!("insert long file: {}", long_file.ident());
                            res.push(Arc::from(FAT32File::from_long(long_file)));
                        }
                        false => {
                            // short file entry
                            let short_file = ShortFile::from(
                                e,
                                Arc::clone(&fat),
                                Arc::clone(&bpb),
                                Arc::clone(&blk),
                            );
                            debug!("insert short file: {}", short_file.ident());
                            res.push(Arc::from(FAT32File::from_short(short_file)));
                        }
                    }
                }
                0x00 => {
                    break;
                }
                _ => {
                    panic!("unknown dir attribute!")
                }
            }
        }
        res
    }
}

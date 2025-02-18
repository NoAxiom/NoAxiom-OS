use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};

use async_trait::async_trait;

use super::{
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    tree::AsNode,
    DirFile,
};
use crate::{
    config::fs::BLOCK_SIZE,
    fs::fat32::{
        cluster_offset_sectors, directory,
        entry::Attribute,
        fat::FAT,
        file::{FAT32File, LongFile, ShortFile},
        ABC,
    },
    include::fs::InodeMode,
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

#[async_trait]
impl AsNode for ShortDirectory {
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
        self.content().await
    }
    async fn content_ref(&self) -> Self::ContentRef {
        self.entry.clusters(&self.blk, &self.fat).await
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

#[async_trait]
impl AsNode for LongDirectory {
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
        self.short_dir.content().await
    }
    async fn content_ref(&self) -> Self::ContentRef {
        self.short_dir
            .entry
            .clusters(&self.short_dir.blk, &self.short_dir.fat)
            .await
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
    fn file_type(&self) -> crate::include::fs::InodeMode {
        crate::include::fs::InodeMode::DIR
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

    fn is_long<S: AsRef<str>>(s: &S) -> bool {
        let s = s.as_ref();
        match s.len() {
            0 => panic!("empty name!"),
            1..=11 => match s.contains(".") {
                true => {
                    let mut s = s.split(".").collect::<Vec<_>>();
                    let ext = s.pop().unwrap();
                    if ext.len() > 3 {
                        true
                    } else {
                        let len = s.iter().fold(0, |acc, &x| acc + x.len());
                        len > 8
                    }
                }
                false => s.len() > 8,
            },
            _ => true,
        }
    }

    async fn create_entry(
        &mut self,
        file: String,
        size: u32,
    ) -> (ShortDirectoryEntry, Arc<FAT>, Arc<[u8; 512]>, Arc<ABC>) {
        let s: String = file.into();
        assert!(!Self::is_long(&s));

        // 短文件名
        let mut name = [0x20; 8];
        let mut extension = [0x20; 3];
        match s.contains(".") {
            true => {
                let mut v = s.split(".").collect::<Vec<_>>();
                let last = v.pop().unwrap();
                extension.copy_from_slice(last.as_bytes());
                for (idx, c) in v.iter().flat_map(|ss| ss.chars()).enumerate() {
                    name[idx] = c as u8;
                }
            }
            false => name[0..s.len()].copy_from_slice(s.as_bytes()),
        }
        if let Some(fst_cluster) = self
            .inner
            .short_dir
            .fat
            .find_free_cluster_id(&self.inner.short_dir.blk)
            .await
        {
            // 标记 `fat` 表为已占用
            self.inner
                .short_dir
                .fat
                .set_cluster_id(&*self.inner.short_dir.blk, fst_cluster, 0xfffffff)
                .await;
            let mut last = fst_cluster;
            // 分配足够的 `FAT` 表项
            for _ in 0..size - 1 {
                let new_cluster = self
                    .inner
                    .short_dir
                    .fat
                    .find_free_cluster_id(&self.inner.short_dir.blk)
                    .await
                    .map_or_else(|| panic!("no avaiable space!"), |x| x);
                self.inner
                    .short_dir
                    .fat
                    .set_cluster_id(&*self.inner.short_dir.blk, last, new_cluster)
                    .await;
                last = new_cluster;
            }
            // 更新最后一项 `FAT` 表
            self.inner
                .short_dir
                .fat
                .set_cluster_id(&self.inner.short_dir.blk, last, 0xfffffff)
                .await;
            let first_cluster_low = fst_cluster as u16;
            let first_cluster_high = (fst_cluster >> 16) as u16;
            let entry = ShortDirectoryEntry {
                name,
                extension,
                attribute: Attribute::ATTR_ARCHIVE,
                _reserved: 0,
                file_size: size * BLOCK_SIZE as u32,
                first_cluster_low,
                first_cluster_high,
                ..Default::default()
            };
            // 下面将 entry 写入块设备
            // 获取父节点结点占用的块号
            let clusters = self.inner.content_ref().await;
            let mut has_free = false;
            let mut free_entry = (0, 0);
            for cluster in &clusters {
                // 获取块号对应的扇区偏移
                let sector = cluster_offset_sectors(&*self.inner.short_dir.bpb, *cluster);
                let block = self.inner.short_dir.blk.read_sector(sector as usize).await;
                let block = block.read().data;
                for (idx, fat) in block.chunks(32).enumerate() {
                    if fat.iter().all(|b| *b == 0x0) {
                        has_free = true;
                        free_entry = (sector, idx);
                        break;
                    }
                }
                if has_free {
                    break;
                }
            }
            if has_free {
                // 如果有空的 `FAT` 表项
                let block = self
                    .inner
                    .short_dir
                    .blk
                    .read_sector(free_entry.0 as usize)
                    .await;
                let mut block = block.read().data;
                for (idx, e) in block.chunks_mut(32).enumerate() {
                    if idx == free_entry.1 {
                        let new_e: [u8; 32] = entry.clone().as_slice();
                        e.copy_from_slice(&new_e);
                        break;
                    }
                }
                // 写回块设备
                self.inner
                    .short_dir
                    .blk
                    .write_sector(free_entry.0 as usize, &block)
                    .await;
            } else {
                // 如果父节点占据的块里面所有目录项都被占用了，则需要申请新的块
                if let Some(new_cluster) = self
                    .inner
                    .short_dir
                    .fat
                    .find_free_cluster_id(&self.inner.short_dir.blk)
                    .await
                {
                    // 父节点最后的块号
                    let last = *clusters.last().unwrap();
                    // 更新 `FAT` 表
                    self.inner
                        .short_dir
                        .fat
                        .set_cluster_id(&self.inner.short_dir.blk, last, new_cluster)
                        .await;
                    self.inner
                        .short_dir
                        .fat
                        .set_cluster_id(&self.inner.short_dir.blk, new_cluster, 0xfffffff)
                        .await;
                    // 将新的块读取进内存
                    let sector = cluster_offset_sectors(&*self.inner.short_dir.bpb, new_cluster);
                    let block = self.inner.short_dir.blk.read_sector(sector as usize).await;
                    let mut block = block.read().data;
                    // 设置第一项的值
                    let e: [u8; 32] = entry.clone().as_slice();
                    block[0..32].copy_from_slice(&e);
                    // 写回块设备
                    self.inner
                        .short_dir
                        .blk
                        .write_sector(sector as usize, &block)
                        .await;
                } else {
                    panic!("no avaiable space in disk!")
                }
            }
            (
                entry,
                Arc::clone(&self.inner.short_dir.fat),
                self.inner.short_dir.bpb.clone(),
                Arc::clone(&self.inner.short_dir.blk),
            )
        } else {
            panic!("no avaiable space in disk!")
        }
    }

    pub async fn create_file(&mut self, file: String, size: u32) -> FAT32File {
        let (entry, fat, bpb, blk) = self.create_entry(file, size).await;
        FAT32File::from_short(ShortFile::from(entry, fat, bpb, blk))
    }

    pub async fn create_dir(&mut self, file: String, size: u32) -> FAT32Directory {
        let (entry, fat, bpb, blk) = self.create_entry(file, size).await;
        debug!("[FAT32] create dir success");
        FAT32Directory::from_short(ShortDirectory::from(entry, fat, bpb, blk))
    }
}

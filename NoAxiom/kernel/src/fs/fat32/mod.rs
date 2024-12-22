//! reference: https://d1.amobbs.com/bbs_upload782111/files_7/armok01151038.pdf

mod bpb;
mod fat;
mod filetree;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use bpb::BIOSParameterBlockOffset;
use filetree::{
    directory::{LongDirectory, ShortDirectory},
    entry::{LongDirectoryEntry, ShortDirectoryEntry},
    file::{LongFile, ShortFile},
    fs_node::FSNode,
    FileTree,
};

use super::{
    blockcache::{AsyncBlockCache, CacheData},
    blockdevice::BlockDevice,
};
use crate::{
    config::fs::{FAT32_SECTOR_SIZE, FIRST_CLUSTER, ROOT_FAKE_ENTRY},
    print, println,
    utils::reverse,
};

type ABC = AsyncBlockCache<CacheData>;

pub struct FAT32FIleSystem {
    blk: Arc<ABC>,
    bpb: [u8; FAT32_SECTOR_SIZE],
    fat: Arc<fat::FAT>,
    /// File ident: string, File content: Vec<u8>
    file_tree: FileTree<String, Vec<u8>>,
    root: String,
}

impl FAT32FIleSystem {
    pub async fn init(device: Arc<dyn BlockDevice>) -> Self {
        let bpb = {
            let mut sector = [0u8; FAT32_SECTOR_SIZE]; // todo: use vec
            let _ = device.read(0, &mut sector).await;
            sector
        };
        // normally, root cluster is 2
        let root_cluster = BIOSParameterBlockOffset::root_cluster(&bpb);
        assert_eq!(root_cluster, FIRST_CLUSTER);

        let blk = Arc::new(AsyncBlockCache::from(device));
        let fat = Arc::new(fat::FAT::new(&bpb));
        let bpb = Arc::new(bpb);

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
        let root_name = root.ident();
        info!("root: {}", root_name);

        // construct the file tree
        let mut file_tree = FileTree::from(Box::new(root.clone())).await;

        let mut dirs: Vec<Box<dyn FSNode<String, Vec<u8>>>> = vec![Box::new(root)];
        let mut long = false;
        let mut long_entries = Vec::new();

        while let Some(directory) = dirs.pop() {
            let content = directory.content().await;
            info!("load dir: {}", directory.ident());
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
                                dirs.push(Box::new(long_dir.clone()));
                                let result =
                                    file_tree.insert(&directory.ident(), Box::new(long_dir));
                                assert!(result.is_ok());
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
                                dirs.push(Box::new(short_dir.clone()));
                                let result =
                                    file_tree.insert(&directory.ident(), Box::new(short_dir));
                                assert!(result.is_ok());
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
                                let result =
                                    file_tree.insert(&directory.ident(), Box::new(long_file));
                                assert!(result.is_ok());
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
                                let result =
                                    file_tree.insert(&directory.ident(), Box::new(short_file));
                                assert!(result.is_ok());
                            }
                        }
                    }
                    0x00 => {
                        info!("end of directory");
                        break;
                    }
                    _ => {
                        panic!("unknown dir attribute!")
                    }
                }
            }
        }

        Self {
            blk,
            bpb: *bpb,
            fat,
            file_tree,
            root: root_name,
        }
    }

    pub async fn list(&self) {
        let result = self.file_tree.list(&self.root);
        match result {
            Ok(res) => {
                println!("> ls");
                res.iter().for_each(|node| {
                    print!("{}  ", node.ident());
                });
                println!("");
            }
            Err(e) => {
                error!("Error listing files: {:?}", e);
            }
        }
    }
}

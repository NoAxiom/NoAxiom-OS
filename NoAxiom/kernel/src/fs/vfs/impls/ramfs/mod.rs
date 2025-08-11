use alloc::{string::String, vec::Vec};

use hashbrown::HashMap;
use ksync::mutex::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod dentry;
pub mod file;
pub mod inode;

lazy_static::lazy_static! {
    static ref RAMFS: RwLock<RamFileManager> = RwLock::new(RamFileManager::new());
}

pub struct RamFileManager {
    contents: HashMap<String, Vec<u8>>,
}

impl RamFileManager {
    pub fn new() -> Self {
        Self {
            contents: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: String) {
        self.contents.insert(path, Vec::new());
    }

    pub fn get_content(&self, path: &str) -> Option<&Vec<u8>> {
        self.contents.get(path)
    }

    pub fn get_content_mut(&mut self, path: &str) -> Option<&mut Vec<u8>> {
        self.contents.get_mut(path)
    }

    pub fn remove_file(&mut self, path: &str) -> Option<Vec<u8>> {
        self.contents.remove(path)
    }
}

pub fn ramfs_read_guard() -> RwLockReadGuard<'static, RamFileManager> {
    RAMFS.read()
}
pub fn ramfs_write_guard() -> RwLockWriteGuard<'static, RamFileManager> {
    RAMFS.write()
}

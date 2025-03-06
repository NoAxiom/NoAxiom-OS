use alloc::{collections::btree_map::BTreeMap, string::String, sync::Arc};

use ksync::mutex::SpinLock;

use super::vfs::basic::filesystem::FileSystem;

lazy_static::lazy_static! {
    pub static ref FS_MANAGER: FileSystemManager = FileSystemManager::new();
}

pub struct FileSystemManager {
    fs_map: SpinLock<BTreeMap<String, Arc<dyn FileSystem>>>,
}

impl FileSystemManager {
    pub fn new() -> Self {
        Self {
            fs_map: SpinLock::new(BTreeMap::new()),
        }
    }

    pub fn register(&self, fs: Arc<dyn FileSystem>) {
        let mut fs_map = self.fs_map.lock();
        fs_map.insert(fs.meta().name.clone(), fs);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn FileSystem>> {
        let fs_map = self.fs_map.lock();
        fs_map.get(name).cloned()
    }
}

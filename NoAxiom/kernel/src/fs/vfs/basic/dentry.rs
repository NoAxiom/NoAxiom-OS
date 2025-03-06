use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use async_trait::async_trait;
use downcast_rs::DowncastSync;
type Mutex<T> = ksync::mutex::SpinLock<T>;

use super::{
    file::File,
    inode::Inode,
    superblock::{EmptySuperBlock, SuperBlock},
};
use crate::{
    fs::path::Path,
    include::{fs::InodeMode, result::Errno},
};

pub struct DentryMeta {
    // todo: dentry states
    /// The name of the dentry
    name: String,

    /// The super block of the dentry
    pub super_block: Arc<dyn SuperBlock>,

    /// The parent of the dentry, None if it is root
    parent: Option<Weak<dyn Dentry>>,
    /// The children of the dentry
    children: Mutex<BTreeMap<String, Arc<dyn Dentry>>>,
    /// The inode of the dentry, None if it is negative
    inode: Mutex<Option<Arc<dyn Inode>>>,
}

impl DentryMeta {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        let inode = Mutex::new(None);
        Self {
            name: name.to_string(),
            super_block,
            parent: parent.map(|p| Arc::downgrade(&p)),
            children: Mutex::new(BTreeMap::new()),
            inode,
        }
    }
}

#[async_trait]
pub trait Dentry: Send + Sync + DowncastSync {
    /// Get the meta of the dentry
    fn meta(&self) -> &DentryMeta;
    /// Open the file associated with the dentry
    fn open(self: Arc<Self>) -> Result<Arc<dyn File>, Errno>;
    /// Get new dentry from name
    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry>;
    /// Create a new dentry with `name` and `mode`
    async fn create(self: Arc<Self>, name: &str, mode: InodeMode)
        -> Result<Arc<dyn Dentry>, Errno>;

    /// Get the inode of the dentry
    fn inode(&self) -> Result<Arc<dyn Inode>, Errno> {
        self.meta()
            .inode
            .lock()
            .as_ref()
            .ok_or(Errno::ENOENT)
            .cloned()
    }
    /// Get the name of the dentry
    fn name(&self) -> String {
        self.meta().name.clone()
    }
    /// Set the inode of the dentry
    fn set_inode(&self, inode: Arc<dyn Inode>) {
        if self.meta().inode.lock().is_some() {
            warn!("replace inode in {:?}", self.name());
        }
        *self.meta().inode.lock() = Some(inode);
    }
}

impl dyn Dentry {
    /// Check if the dentry is negative
    pub fn is_negetive(&self) -> bool {
        self.inode().is_err()
    }

    /// Create a negetive child dentry with `name`.
    pub fn new_child(self: &Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let child = self.clone().from_name(name);
        child
    }

    /// Get the parent of the dentry
    pub fn parent(self: &Arc<Self>) -> Option<Arc<dyn Dentry>> {
        self.meta().parent.as_ref().and_then(|p| p.upgrade())
    }

    /// Get the children of the dentry
    pub fn children(&self) -> BTreeMap<String, Arc<dyn Dentry>> {
        self.meta().children.lock().clone()
    }

    /// Add a child dentry with `name` and `child_inode`, for realfs only.
    pub fn add_child(self: &Arc<Self>, name: &str, child_inode: Arc<dyn Inode>) -> Arc<dyn Dentry> {
        let mut children = self.meta().children.lock();

        if let Some(child) = children.get(name) {
            child.set_inode(child_inode);
            child.clone()
        } else {
            let child = self.new_child(name);
            child.set_inode(child_inode);
            children.insert(name.to_string(), child.clone());
            child
        }
    }

    /// Remove a child dentry with `name`.
    pub fn remove_child(self: &Arc<Self>, name: &str) {
        self.meta().children.lock().remove(name);
    }

    /// Add a child to directory dentry with `name` and `mode`, for syscall
    /// only.
    pub async fn add_dir_child(
        self: &Arc<Self>,
        name: &str,
        mode: &InodeMode,
    ) -> Result<Arc<dyn Dentry>, Errno> {
        if self.inode().unwrap().file_type() != InodeMode::DIR {
            return Err(Errno::ENOTDIR);
        }
        let child = self.clone().create(name, *mode).await?;
        self.meta()
            .children
            .lock()
            .insert(name.to_string(), child.clone());
        Ok(child)
    }

    /// Get super block of the dentry
    pub fn super_block(&self) -> Arc<dyn SuperBlock> {
        self.meta().super_block.clone()
    }

    /// Get the path of the dentry
    pub fn path(self: Arc<Self>) -> Path {
        let mut path = self.name();
        let mut current = self.clone();
        while let Some(parent) = current.parent() {
            path = format!("{}/{}", parent.name(), path);
            current = parent;
        }
        if path.len() > 1 {
            path.remove(0);
        }
        Path::from(path)
    }

    /// Find the dentry with `name` in the **WHOLE** sub-tree of the dentry.
    pub fn find(self: Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        if self.name() == name {
            return Some(self.clone());
        }

        for child in self.meta().children.lock().values() {
            if child.name() == name {
                return Some(child.clone());
            }

            if let Ok(inode) = child.inode() {
                if inode.file_type() == InodeMode::DIR {
                    if let Some(d) = child.clone().find(name) {
                        return Some(d);
                    }
                }
            }
        }
        None
    }

    /// Find the dentry with `path`, Error if not found.
    pub fn find_path(self: Arc<Self>, path: &Vec<&str>) -> Result<Arc<dyn Dentry>, Errno> {
        let mut idx = 0;
        let max_idx = path.len() - 1;
        let mut current = self.clone();

        while idx <= max_idx {
            let name = path[idx];
            if name.is_empty() || name == "." {
                idx += 1;
                continue;
            }

            if let Some(child) = current.clone().meta().children.lock().get(name) {
                if idx < max_idx {
                    let inode = child.inode()?;
                    assert!(inode.file_type() == InodeMode::DIR);
                }
                current = child.clone();
                idx += 1;
            } else {
                return Err(Errno::ENOENT);
            }
        }

        Ok(current)
    }

    /// Find the dentry with `path`, create negative dentry if not found.
    pub fn find_path_or_create(self: Arc<Self>, path: &Vec<&str>) -> Arc<dyn Dentry> {
        let mut idx = 0;
        let max_idx = path.len() - 1;
        let mut current = self.clone();
        while idx <= max_idx {
            let name = path[idx];
            if name.is_empty() {
                idx += 1;
                continue;
            }

            let current_clone = current.clone();
            let meta = current_clone.meta();
            let mut children = meta.children.lock();

            if let Some(child) = children.get(name) {
                if idx < max_idx {
                    let inode = child.inode().unwrap();
                    assert!(inode.file_type() == InodeMode::DIR);
                }
                current = child.clone();
                idx += 1;
            } else {
                let new_child = current.clone().new_child(name);
                children.insert(name.to_string(), new_child.clone());
                current = new_child;
                idx += 1;
            }
        }
        current
    }
}

pub struct EmptyDentry {
    meta: DentryMeta,
}

impl EmptyDentry {
    pub fn new() -> Self {
        let super_block = Arc::new(EmptySuperBlock::new());
        Self {
            meta: DentryMeta::new(None, "", super_block),
        }
    }
}

#[async_trait]
impl Dentry for EmptyDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn open(self: Arc<Self>) -> Result<Arc<dyn File>, Errno> {
        unreachable!()
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!()
    }

    async fn create(
        self: Arc<Self>,
        _name: &str,
        _mode: InodeMode,
    ) -> Result<Arc<dyn Dentry>, Errno> {
        unreachable!()
    }
}

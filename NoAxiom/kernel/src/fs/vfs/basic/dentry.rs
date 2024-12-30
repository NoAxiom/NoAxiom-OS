use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
};

use super::{file::File, inode::Inode, superblock::SuperBlock};
use crate::{
    nix::{fs::InodeMode, result::Errno},
    sync::mutex::SpinLock,
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
    children: SpinLock<BTreeMap<String, Arc<dyn Dentry>>>,
    /// The inode of the dentry, None if it is negative
    inode: SpinLock<Option<Arc<dyn Inode>>>,
}

impl DentryMeta {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        let inode = SpinLock::new(None);
        Self {
            name: name.to_string(),
            super_block,
            parent: parent.map(|p| Arc::downgrade(&p)),
            children: SpinLock::new(BTreeMap::new()),
            inode,
        }
    }
}

pub trait Dentry: Send + Sync {
    /// Get the meta of the dentry
    fn meta(&self) -> &DentryMeta;
    /// Open the file associated with the dentry
    fn open(self: Arc<Self>) -> Result<Arc<dyn File>, Errno>;
    /// Get new dentry from name
    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry>;

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
    /// Create a negetive child dentry with `name`.
    pub fn new_child(self: &Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let child = self.clone().from_name(name);
        child
    }

    pub fn add_child(self: &Arc<Self>, name: &str, child_inode: Arc<dyn Inode>) -> Arc<dyn Dentry> {
        let child = self.new_child(name);
        child.set_inode(child_inode);
        self.meta()
            .children
            .lock()
            .insert(name.to_string(), child.clone());
        child
    }
    pub fn super_block(&self) -> Arc<dyn SuperBlock> {
        self.meta().super_block.clone()
    }
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
}

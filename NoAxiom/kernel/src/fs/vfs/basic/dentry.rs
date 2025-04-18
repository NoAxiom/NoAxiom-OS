use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};

use async_trait::async_trait;
use downcast_rs::DowncastSync;
use ksync::mutex::check_no_lock;
use spin::Mutex;

use super::{
    file::File,
    inode::Inode,
    superblock::{EmptySuperBlock, SuperBlock},
};
use crate::{
    fs::path::Path,
    include::{fs::InodeMode, result::Errno},
    syscall::SyscallResult,
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
    /// Now it holds the inode of the file the whole life time
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
            assert_eq!(
                inode.file_type(),
                self.inode().unwrap().file_type(),
                "{}",
                format!(
                    "replace inode in {}, type: {:?}",
                    self.name(),
                    self.inode().unwrap().file_type()
                )
            );
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
    pub fn children(&self) -> spin::MutexGuard<BTreeMap<String, Arc<dyn Dentry>>> {
        self.meta().children.lock()
    }

    pub fn get_child(&self, name: &str) -> Option<Arc<dyn Dentry>> {
        self.meta().children.lock().get(name).cloned()
    }

    /// use self.create() to generate child dentry
    /// Add a child dentry with `name` and `child_inode`, for realfs only.
    pub fn add_child(self: &Arc<Self>, name: &str, child_inode: Arc<dyn Inode>) -> Arc<dyn Dentry> {
        let mut children = self.meta().children.lock();

        let res = if let Some(child) = children.get(name) {
            child.set_inode(child_inode);
            child.clone()
        } else {
            let child = self.new_child(name);
            child.set_inode(child_inode);
            children.insert(name.to_string(), child.clone());
            child
        };
        res
    }

    /// use child dentry directly
    /// Add a child dentry with `child` directly, for fs which doesn't
    /// support create or used in different type fs (like mount). Basicly like
    /// `add_dir_child`.
    pub fn add_child_directly(self: &Arc<Self>, child: Arc<dyn Dentry>) {
        let mut children = self.meta().children.lock();
        let name = child.name();
        if let Some(old) = children.insert(name.clone(), child) {
            warn!(
                "add child {} to {} already has child {}, replace it",
                name,
                self.name(),
                old.name()
            );
        }
    }

    // pub fn delete_self(self: &Arc<Self>) {
    //     if let Some(parent) = self.parent() {
    //         parent.remove_child(&self.name());
    //     }
    //     let mut inode = self.meta().inode.lock();
    //     if let Some(inode_arc) = inode.take() {
    //         drop(inode_arc);
    //         *inode = None;
    //     }
    // }

    /// Remove a child dentry with `name`.
    pub fn remove_child(self: &Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        self.meta().children.lock().remove(name)
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
        Path::try_from(path).unwrap()
    }

    /// Find the dentry with `name` in the **WHOLE** sub-tree of the dentry.
    #[allow(unused)]
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
                println!("[kernel] [find_path] file not exist");
                return Err(Errno::ENOENT);
            }
        }

        Ok(current)
    }

    /// Find the dentry with `path`, create negative dentry if not found.
    /// if just open a file, and the path is invalid, all the path dentry will
    /// be created and will write to the disk. If the path isvalid, or `create`
    /// the file, it's ok.
    ///
    /// todo: path cache
    pub async fn find_path_or_create(
        self: Arc<Self>,
        path: &Vec<&str>,
        mode: InodeMode,
    ) -> Arc<dyn Dentry> {
        let mut idx = 0;
        let max_idx = path.len() - 1;
        let mut current = self.clone();
        while idx <= max_idx {
            let name = path[idx];
            if name.is_empty() {
                idx += 1;
                continue;
            }
            assert!(current.clone().inode().unwrap().file_type() == InodeMode::DIR);
            if current.clone().children().is_empty() {
                if let Ok(current_dir) = current.clone().open() {
                    assert!(check_no_lock());
                    current_dir.load_dir().await.unwrap();
                }
            }
            if let Some(child) = current.clone().get_child(name) {
                current = child.clone();
                idx += 1;
                continue;
            }

            if idx < max_idx {
                debug!("[find_path_or_create] create dir {}", name);
                assert!(check_no_lock());
                current = current.create(name, InodeMode::DIR).await.unwrap();
            } else {
                debug!("[find_path_or_create] create file {}", name);
                assert!(check_no_lock());
                current = current.create(name, mode).await.unwrap();
            }
            idx += 1;
        }
        current
    }

    /// Hard link, link self to `target`.
    pub fn link_to(self: Arc<Self>, target: Arc<dyn Dentry>) -> Result<Arc<dyn Dentry>, Errno> {
        if !self.is_negetive() {
            return Err(Errno::EEXIST);
        }
        let inode = target.inode()?;
        inode.meta().inner.lock().nlink += 1;
        self.set_inode(inode);
        Ok(self)
    }

    /// Unlink, unlink self and delete the inner file if nlink is 0.
    pub async fn unlink(self: Arc<Self>) -> SyscallResult {
        let inode = self.inode()?;
        let mut nlink = inode.meta().inner.lock().nlink;
        nlink -= 1;
        if nlink == 0 {
            let parent = self.parent().unwrap();
            if parent.inode()?.file_type() != InodeMode::DIR {
                return Err(Errno::ENOTDIR);
            }
            parent.remove_child(&self.name()).unwrap();
            parent.open().unwrap().delete_child(&self.name()).await?;
        }
        Ok(0)
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

use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::panic;

use async_trait::async_trait;
use downcast_rs::DowncastSync;
use ksync::mutex::{check_no_lock, SpinLock, SpinLockGuard};

type Mutex<T> = SpinLock<T>;
type MutexGuard<'a, T> = SpinLockGuard<'a, T>;

use super::{
    file::File,
    inode::Inode,
    superblock::{EmptySuperBlock, SuperBlock},
};
use crate::{
    fs::{path::Path, vfs::basic::inode::InodeState},
    include::{
        fs::{InodeMode, RenameFlags},
        result::Errno,
    },
    sched::utils::block_on,
    syscall::{SysResult, SyscallResult},
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
    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>>;
    /// Get new dentry from name
    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry>;
    /// Create a new dentry with `name` and `mode`
    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>>;
    /// Get the inode of the dentry
    fn inode(&self) -> SysResult<Arc<dyn Inode>> {
        if let Some(inode) = self.meta().inode.lock().as_ref() {
            Ok(inode.clone())
        } else {
            warn!("[kernel] [dentry] {} inode not exist", self.name());
            Err(Errno::ENOENT)
        }
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
            return;
        }
        *self.meta().inode.lock() = Some(inode);
    }
    fn set_inode_none(&self) {
        *self.meta().inode.lock() = None;
    }
}

impl dyn Dentry {
    /// Check if the dentry is negative
    pub fn is_negative(&self) -> bool {
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
    pub fn children(&self) -> MutexGuard<BTreeMap<String, Arc<dyn Dentry>>> {
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

    /// Get the path of the dentry, panic if the dentry is deleted.
    /// todo: this function cost a lot, we should add PathCache
    pub fn path(self: Arc<Self>) -> SysResult<Path> {
        let mut path = self.name();
        let mut current = self.clone();
        while let Some(parent) = current.parent() {
            path = format!("{}/{}", parent.name(), path);
            current = parent;
        }
        if path.len() > 1 {
            path.remove(0);
        }
        assert!(path.starts_with('/'));
        Path::try_from(path)
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
    ///
    /// - all the mid dentry should be NON-NEGATIVE and DIR
    /// - the file dentry can be negative
    pub fn find_path(self: Arc<Self>, path: &Vec<&str>) -> SysResult<Arc<dyn Dentry>> {
        let mut idx = 0;
        let max_idx = path.len() - 1;
        let mut current = self.clone();

        while idx <= max_idx {
            let name = path[idx];
            if name.is_empty() || name == "." {
                idx += 1;
                continue;
            }
            assert!(current.clone().inode().unwrap().file_type() == InodeMode::DIR);
            if current.clone().children().is_empty() {
                if let Ok(current_dir) = current.clone().open() {
                    warn!(
                        "[find_path] the {} is not open! Now open it.",
                        current.name()
                    );
                    assert_no_lock!();
                    block_on(current_dir.load_dir()).unwrap();
                }
            }
            if let Some(child) = current.clone().meta().children.lock().get(name) {
                if idx < max_idx {
                    let inode = child.inode()?;
                    let file_type = inode.file_type();
                    if file_type != InodeMode::DIR {
                        error!(
                            "[kernel] [find_path] {} which is {:?} is not a dir",
                            name, file_type
                        );
                        return Err(Errno::ENOTDIR);
                    }
                }
                current = child.clone();
                idx += 1;
            } else {
                #[cfg(feature = "debug_sig")]
                {
                    let path_str = path.join("/");
                    warn!("[kernel] [find_path] {} not exist", path_str);
                }
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
        use arch::ArchInt;

        let mut idx = 0;
        let max_idx = path.len() - 1;
        let mut current = self.clone();
        while idx <= max_idx {
            let name = path[idx];
            if name.is_empty() || name == "." {
                idx += 1;
                continue;
            }
            assert!(current.clone().inode().unwrap().file_type() == InodeMode::DIR);
            if current.clone().children().is_empty() {
                if let Ok(current_dir) = current.clone().open() {
                    assert_no_lock!();
                    current_dir.load_dir().await.unwrap();
                }
            }
            if let Some(child) = current.clone().get_child(name) {
                // unlikely
                if child.is_negative() {
                    warn!("[find_path_or_create] {} is negative", child.name());
                    current = current.create(name, mode).await.unwrap();
                } else {
                    current = child.clone();
                }
                idx += 1;
                continue;
            }

            if idx < max_idx {
                debug!("[find_path_or_create] create dir {}", name);
                assert_no_lock!();
                assert!(arch::Arch::is_interrupt_enabled());
                current = current.create(name, InodeMode::DIR).await.unwrap();
            } else {
                debug!("[find_path_or_create] create file {}", name);
                assert_no_lock!();
                assert!(arch::Arch::is_interrupt_enabled());
                current = current.create(name, mode).await.unwrap();
            }
            idx += 1;
        }
        current
    }

    /// Hard link, link self to `target`.
    pub fn link_to(self: Arc<Self>, target: Arc<dyn Dentry>) -> SysResult<Arc<dyn Dentry>> {
        if !self.is_negative() {
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
        debug!(
            "[Vfs::unlink] nlink: {}, file_type: {:?}",
            nlink,
            inode.file_type()
        );
        nlink -= 1;
        if nlink == 0 {
            let parent = self.parent().unwrap();
            if parent.inode()?.file_type() != InodeMode::DIR {
                return Err(Errno::ENOTDIR);
            }
            self.set_inode_none();
            inode.set_state(InodeState::Deleted);
            // parent.remove_child(&self.name()).unwrap();
            parent.open().unwrap().delete_child(&self.name()).await?;
        }
        Ok(0)
    }

    pub async fn rename_to(
        self: Arc<Self>,
        target: Arc<dyn Dentry>,
        flags: RenameFlags,
    ) -> SysResult<()> {
        if flags.contains(RenameFlags::RENAME_EXCHANGE)
            && (flags.contains(RenameFlags::RENAME_NOREPLACE)
                || flags.contains(RenameFlags::RENAME_WHITEOUT))
        {
            return Err(Errno::EINVAL);
        }
        // FIXME: i don't think should check if descendant
        if target.is_negative() && flags.contains(RenameFlags::RENAME_EXCHANGE) {
            return Err(Errno::ENOENT);
        } else if flags.contains(RenameFlags::RENAME_NOREPLACE) {
            return Err(Errno::EEXIST);
        }

        // ext4_rs doesn't support mv or rename methods
        // so we delete the target and copy from self

        let tar_parent = target.parent().unwrap_or_else(|| {
            panic!("rename_to: target parent is None");
        });
        let self_file_type = self.inode()?.file_type();
        let tar_name = target.name();

        // delete
        if !target.is_negative() {
            let tar_file_type = target.inode()?.file_type();
            if self_file_type != tar_file_type {
                return Err(Errno::EINVAL);
            }
            match tar_file_type {
                InodeMode::DIR => {
                    warn!("[rename_to] delete dir {}", target.name());
                }
                InodeMode::FILE => {}
                _ => unimplemented!("rename at not dir/file"),
            }
            tar_parent
                .clone()
                .open()?
                .delete_child(tar_name.as_str())
                .await?;
        }

        let target = tar_parent.remove_child(target.name().as_str()).unwrap();

        // copy
        tar_parent
            .create(self.name().as_str(), self_file_type)
            .await?;

        if flags.contains(RenameFlags::RENAME_EXCHANGE) {
            self.set_inode(target.inode()?);
        } else {
            self.set_inode_none();
        }

        Ok(())
    }
}

pub struct EmptyDentry {
    meta: DentryMeta,
}

impl EmptyDentry {
    pub fn new(name: &str) -> Self {
        let super_block = Arc::new(EmptySuperBlock::new());
        Self {
            meta: DentryMeta::new(None, name, super_block),
        }
    }
}

#[async_trait]
impl Dentry for EmptyDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        unreachable!()
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!()
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!()
    }
}

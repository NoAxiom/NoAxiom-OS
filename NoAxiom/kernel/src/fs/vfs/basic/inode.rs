use alloc::{string::String, sync::Arc};

use downcast_rs::{impl_downcast, DowncastSync};
type Mutex<T> = ksync::mutex::SpinLock<T>;

use super::{file::File, superblock::SuperBlock};
use crate::nix::{
    fs::{InodeMode, Stat},
    result::Errno,
};

lazy_static::lazy_static! {
    static ref INODE_ID: Mutex<usize> = Mutex::new(0);
}
fn alloc_id() -> usize {
    let mut id = INODE_ID.lock();
    *id += 1;
    *id
}

pub enum InodeState {
    UnInit,
    Normal,
    Dirty,
}

pub struct InodeMeta {
    /// The inode id, unique in the file system
    id: usize,
    /// The inner data of the inode, maybe modified by multiple tasks
    inner: Mutex<InodeMetaInner>,
    /// The mode of file
    inode_mode: InodeMode,
    /// The super block of the inode
    super_block: Arc<dyn SuperBlock>,
}

impl InodeMeta {
    pub fn new(super_block: Arc<dyn SuperBlock>, inode_mode: InodeMode, size: usize) -> Self {
        Self {
            id: alloc_id(),
            inner: Mutex::new(InodeMetaInner {
                nlink: 1,
                size,
                state: InodeState::UnInit,
            }),
            inode_mode,
            super_block,
        }
    }
}

pub struct InodeMetaInner {
    /// The number of links to the inode
    nlink: usize,
    /// The size of the file
    size: usize,
    /// The state of the file
    state: InodeState,
}

pub trait Inode: Send + Sync + DowncastSync {
    fn meta(&self) -> &InodeMeta;
    fn stat(&self) -> Result<Stat, Errno>;
}

impl dyn Inode {
    pub fn id(&self) -> usize {
        self.meta().id
    }
    pub fn size(&self) -> usize {
        self.meta().inner.lock().size
    }
    pub fn file_type(&self) -> InodeMode {
        self.meta().inode_mode
    }
}

impl_downcast!(sync Inode);

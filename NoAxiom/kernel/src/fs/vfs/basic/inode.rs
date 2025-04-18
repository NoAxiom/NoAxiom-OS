use alloc::sync::Arc;

use downcast_rs::{impl_downcast, DowncastSync};
use spin::Mutex;

use super::superblock::{EmptySuperBlock, SuperBlock};
use crate::{
    include::{
        fs::{InodeMode, Stat},
        result::Errno,
    },
    time::time_spec::TimeSpec,
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
    pub id: usize,
    /// The inner data of the inode, maybe modified by multiple tasks
    pub inner: Mutex<InodeMetaInner>,
    /// The mode of file
    pub inode_mode: InodeMode,
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
                atime_sec: 0,
                atime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                ctime_sec: 0,
                ctime_nsec: 0,
            }),
            inode_mode,
            super_block,
        }
    }
}

pub struct InodeMetaInner {
    /// The number of links to the inode
    pub nlink: usize,
    /// The size of the file
    pub size: usize,
    /// The state of the file
    state: InodeState,
    /// Last access time.
    pub atime_sec: usize,
    pub atime_nsec: usize,
    /// Last modification time.
    pub mtime_sec: usize,
    pub mtime_nsec: usize,
    /// Last status change time.
    pub ctime_sec: usize,
    pub ctime_nsec: usize,
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
    pub fn set_size(&self, size: usize) {
        self.meta().inner.lock().size = size;
    }
    // set timestamp, `None` means not to change
    pub fn set_time(
        &self,
        atime: &Option<TimeSpec>,
        mtime: &Option<TimeSpec>,
        ctime: &Option<TimeSpec>,
    ) {
        let mut inner = self.meta().inner.lock();
        if let Some(atime) = atime {
            inner.atime_sec = atime.tv_sec;
            inner.atime_nsec = atime.tv_nsec;
        }
        if let Some(mtime) = mtime {
            inner.mtime_sec = mtime.tv_sec;
            inner.mtime_nsec = mtime.tv_nsec;
        }
        if let Some(ctime) = ctime {
            inner.ctime_sec = ctime.tv_sec;
            inner.ctime_nsec = ctime.tv_nsec;
        }
    }
}

impl_downcast!(sync Inode);

pub struct EmptyInode {
    meta: InodeMeta,
}

impl EmptyInode {
    pub fn new() -> Self {
        let super_block = Arc::new(EmptySuperBlock::new());
        let inode_mode = InodeMode::empty();
        Self {
            meta: InodeMeta::new(super_block, inode_mode, 0),
        }
    }
}

impl Inode for EmptyInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }

    fn stat(&self) -> Result<Stat, Errno> {
        Ok(Stat::default())
    }
}

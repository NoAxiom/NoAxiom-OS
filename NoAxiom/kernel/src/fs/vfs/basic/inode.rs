use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use downcast_rs::{impl_downcast, DowncastSync};
use ksync::{
    async_mutex::{AsyncMutex, AsyncMutexGuard},
    mutex::SpinLock,
};

use super::superblock::{EmptySuperBlock, SuperBlock};
use crate::{
    fs::pagecache::PageCache,
    include::fs::{InodeMode, Stat, Statx, StatxTimestamp},
    syscall::SysResult,
    time::time_spec::TimeSpec,
};

type Mutex<T> = SpinLock<T>;

lazy_static::lazy_static! {
    static ref INODE_ID: Mutex<usize> = Mutex::new(0);
}
fn alloc_id() -> usize {
    let mut id = INODE_ID.lock();
    *id += 1;
    *id
}

#[allow(unused)]
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
    pub super_block: Arc<dyn SuperBlock>,
    /// The page cache of the file, managed by the `Inode`
    pub page_cache: Option<AsyncMutex<PageCache>>,
}

impl InodeMeta {
    pub fn new(
        super_block: Arc<dyn SuperBlock>,
        inode_mode: InodeMode,
        size: usize,
        cached: bool,
    ) -> Self {
        let page_cache = if cached {
            Some(AsyncMutex::new(PageCache::new()))
        } else {
            None
        };
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
            page_cache,
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

#[async_trait]
pub trait Inode: Send + Sync + DowncastSync {
    fn meta(&self) -> &InodeMeta;
    fn stat(&self) -> SysResult<Stat>;
    async fn truncate(&self, _new: usize) -> SysResult<()> {
        panic!("this inode not implemented truncate");
    }
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
    pub async fn page_cache(&self) -> Option<AsyncMutexGuard<'_, PageCache>> {
        if let Some(page_cache) = &self.meta().page_cache {
            Some(page_cache.lock().await)
        } else {
            None
        }
    }
    pub fn statx(&self, mask: u32) -> SysResult<Statx> {
        let stat = self.stat()?;
        Ok(Statx::new(
            mask,
            stat.st_nlink,
            stat.st_mode as u16,
            stat.st_ino,
            stat.st_size,
            StatxTimestamp::new(stat.st_atime_sec as i64, stat.st_atime_nsec as u32),
            StatxTimestamp::new(stat.st_ctime_sec as i64, stat.st_ctime_nsec as u32),
            StatxTimestamp::new(stat.st_mtime_sec as i64, stat.st_mtime_nsec as u32),
            (stat.st_rdev as u32 & 0xffff_00) >> 8 as u32,
            (stat.st_rdev & 0xff) as u32,
            (stat.st_dev as u32 & 0xffff_00) >> 8 as u32,
            (stat.st_dev & 0xff) as u32,
        ))
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
            meta: InodeMeta::new(super_block, inode_mode, 0, false),
        }
    }
}

impl Inode for EmptyInode {
    fn meta(&self) -> &InodeMeta {
        &self.meta
    }

    fn stat(&self) -> SysResult<Stat> {
        Ok(Stat::default())
    }
}

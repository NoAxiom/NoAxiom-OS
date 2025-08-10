use alloc::{boxed::Box, string::String, sync::Arc};
use core::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use config::fs::BLOCK_SIZE;
use downcast_rs::{impl_downcast, DowncastSync};
use include::errno::Errno;
use ksync::mutex::SpinLock;

use super::superblock::{EmptySuperBlock, SuperBlock};
use crate::{
    include::{
        fs::{
            InodeMode, Stat, Statx, StatxTimestamp, ALL_PERMISSIONS_MASK, PRIVILEGE_MASK, TYPE_MASK,
        },
        time::TimeSpec,
    },
    syscall::SysResult,
    task::Task,
    utils::global_alloc,
};

type Mutex<T> = SpinLock<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeState {
    UnInit,
    Normal,
    Dirty,
    Deleted,
}

pub struct InodeMeta {
    /// The inode id, unique in the file system
    pub id: usize,
    /// The inner data of the inode, maybe modified by multiple tasks
    pub inner: Mutex<InodeMetaInner>,
    /// The mode of file
    pub inode_mode: AtomicU32,
    /// The super block of the inode
    pub super_block: Arc<dyn SuperBlock>,
    /// The page cache of the file, managed by the `Inode`
    pub page_cache: Option<()>,

    pub uid: AtomicU32,             // todo: add this to the stat
    pub gid: AtomicU32,             // todo: add this to the stat
    symlink: Mutex<Option<String>>, // for symlink, the target path
}

// todo: Drop for the InodeMeta, sync the page cache according to the state

impl InodeMeta {
    pub fn new(
        super_block: Arc<dyn SuperBlock>,
        inode_mode: InodeMode,
        size: usize,
        cached: bool,
    ) -> Self {
        let page_cache = if cached { Some(()) } else { None };
        Self {
            id: global_alloc() as usize,
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
            inode_mode: AtomicU32::new(inode_mode.bits()),
            super_block,
            page_cache,
            uid: AtomicU32::new(0),    // default user id
            gid: AtomicU32::new(0),    // default group id
            symlink: Mutex::new(None), // default no symlink
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
    #[inline(always)]
    pub fn id(&self) -> usize {
        self.meta().id
    }
    #[inline(always)]
    pub fn uid(&self) -> u32 {
        self.meta().uid.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn gid(&self) -> u32 {
        self.meta().gid.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_uid(&self, uid: u32) {
        self.meta().uid.store(uid, Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn set_gid(&self, gid: u32) {
        self.meta().gid.store(gid, Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.meta().inner.lock().size
    }
    #[inline(always)]
    pub fn state(&self) -> InodeState {
        self.meta().inner.lock().state
    }
    #[inline(always)]
    pub fn inode_mode(&self) -> InodeMode {
        InodeMode::from_bits(self.meta().inode_mode.load(Ordering::SeqCst))
            .unwrap_or(InodeMode::empty())
    }
    #[inline(always)]
    pub fn set_inode_mode(&self, mode: InodeMode) {
        self.meta().inode_mode.store(mode.bits(), Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn file_type(&self) -> InodeMode {
        let inode_mode = self.meta().inode_mode.load(Ordering::SeqCst);
        InodeMode::file_type(inode_mode).expect("Invalid file type!")
    }
    #[inline(always)]
    pub fn set_size(&self, size: usize) {
        self.meta().inner.lock().size = size;
    }
    #[inline(always)]
    pub fn symlink(&self) -> Option<String> {
        let symlink = self.meta().symlink.lock();
        symlink.clone()
    }
    #[inline(always)]
    pub fn set_symlink(&self, target: String) {
        let mut symlink = self.meta().symlink.lock();
        *symlink = Some(target);
    }
    pub fn privilege(&self) -> InodeMode {
        let inode_mode = self.meta().inode_mode.load(Ordering::SeqCst);
        let inode_mode = inode_mode & PRIVILEGE_MASK;
        InodeMode::from_bits(inode_mode).expect("Invalid inode privilege!")
    }
    pub fn set_permission(&self, task: &Arc<Task>, mode: u32) {
        const S_ISGID: u32 = InodeMode::SET_GID.bits();
        let inode_mode = self.meta().inode_mode.load(Ordering::SeqCst);
        let mut mode = mode;
        if mode & S_ISGID != 0 {
            let (fsuid, fsgid) = (task.fsuid(), task.fsgid());
            if fsuid != 0 && fsgid != self.gid() as u32 {
                warn!(
                    "[set_permission] S_ISGID clear! fsuid: {}, fsgid: {}, gid: {}",
                    fsuid,
                    fsgid,
                    self.gid()
                );
                mode &= !S_ISGID;
            }
        }
        self.meta().inode_mode.store(
            (inode_mode & !ALL_PERMISSIONS_MASK) | (mode & ALL_PERMISSIONS_MASK),
            Ordering::SeqCst,
        );
    }
    #[inline(always)]
    pub fn page_cache(&self) -> Option<()> {
        self.meta().page_cache
    }
    pub fn statx(&self, mask: u32) -> SysResult<Statx> {
        let stat = self.stat()?;
        Ok(Statx {
            stx_mask: mask,
            stx_blksize: BLOCK_SIZE as u32,
            stx_attributes: 0,
            stx_nlink: stat.st_nlink,
            stx_uid: stat.st_uid,
            stx_gid: stat.st_gid,
            stx_mode: stat.st_mode as u16,
            __statx_pad1: [0; 1],
            stx_ino: stat.st_ino,
            stx_size: stat.st_size,
            stx_blocks: stat.st_blocks,
            stx_attributes_mask: 0,
            stx_atime: StatxTimestamp::new(stat.st_atime_sec as i64, stat.st_atime_nsec as u32),
            stx_ctime: StatxTimestamp::new(stat.st_ctime_sec as i64, stat.st_ctime_nsec as u32),
            stx_btime: StatxTimestamp::new(stat.st_ctime_sec as i64, stat.st_ctime_nsec as u32),
            stx_mtime: StatxTimestamp::new(stat.st_mtime_sec as i64, stat.st_mtime_nsec as u32),
            stx_rdev_major: (stat.st_rdev as u32 & 0xffff_00) >> 8 as u32,
            stx_rdev_minor: (stat.st_rdev & 0xff) as u32,
            stx_dev_major: (stat.st_dev as u32 & 0xffff_00) >> 8 as u32,
            stx_dev_minor: (stat.st_dev & 0xff) as u32,
            stx_mnt_id: 0,
            __statx_pad2: 0,
            __statx_pad3: [0; 12],
        })
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
    pub async fn set_state(&self, state: InodeState) {
        let mut inner = self.meta().inner.lock();
        match inner.state {
            InodeState::Deleted => {}
            _ => {
                inner.state = state;
            }
        }
    }

    /// ref: RocketOS
    /// change the owner and group of a file
    pub fn chown(&self, task: &Arc<Task>, uid: u32, gid: u32) -> SysResult<()> {
        let euid = task.fsuid();
        let egid = task.fsgid();
        let mut mode = self.inode_mode();
        info!(
            "[chown] euid: {}, egid: {}, uid: {}, gid: {}, mode: {:?}",
            euid, egid, uid, gid, mode
        );

        let uid_change = uid != u32::MAX; // -1 means not to change
        let gid_change = gid != u32::MAX; // -1 means not to change

        // ROOT
        if euid == 0 {
            if uid_change {
                if mode.bits() & 0o111 != 0 && mode.contains(InodeMode::FILE) {
                    warn!("[chown] Root User changes the owner");
                    if mode.contains(InodeMode::GROUP_EXEC) {
                        // clear setuid and setgid
                        mode &= !(InodeMode::SET_GID | InodeMode::SET_UID);
                    } else {
                        mode &= !(InodeMode::SET_UID);
                    }
                    self.set_inode_mode(mode);
                }

                self.set_uid(uid);
            }
            if gid_change {
                self.set_gid(gid);
            }
            return Ok(());
        }

        // change uid
        if uid_change {
            // just support root changer owner
            warn!("[chown] User changes the owner, but not root, not supported yet");
            return Err(Errno::EPERM);
        }

        // change gid
        if gid_change && gid != self.gid() {
            warn!("inode gid: {}", self.gid());
            if euid != self.uid() {
                error!(
                    "[chown] No permission to change ownership, euid: {}, egid: {}",
                    euid, egid
                );
                return Err(Errno::EPERM);
            }
            // 检查new_gid是否是当前用户的egid或附属组
            if egid != gid {
                let sup_groups = task.sup_groups();
                if !sup_groups.contains(&gid) {
                    error!("[chown] New group {} is not in the effective groups of task {}, euid: {}, egid: {}", gid, task.tid(), euid, egid);
                    return Err(Errno::EPERM);
                }
            }
            if mode.other_permissions() != 0 && mode.contains(InodeMode::FILE) {
                warn!("[chown] Normal User changes the owner");
                // 如果是文件是non-group-executable, 则保留setgid位
                if mode.contains(InodeMode::GROUP_EXEC) {
                    // clear setuid and setgid
                    mode &= !(InodeMode::SET_GID | InodeMode::SET_UID);
                } else {
                    mode &= !(InodeMode::SET_UID);
                }
                self.set_inode_mode(mode);
            }
            self.set_gid(gid);
        }
        Ok(())
    }

    pub fn check_search_permission(&self, task: &Arc<Task>) -> bool {
        let euid = task.fsuid();
        let egid = task.fsgid();
        if euid == 0 {
            return true;
        }
        let mode = self.inode_mode();
        debug!(
            "[check_search_permission] i_mode: {:o}, euid: {}, egid: {}",
            mode, euid, egid
        );

        let perm = if euid == self.uid() {
            mode.user_permissions()
        } else if egid == self.gid() {
            mode.group_permissions()
        } else {
            mode.other_permissions()
        };

        if perm & 0o111 == 0 {
            error!("[check_search_permission] No search permission !!");
            false
        } else {
            true
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

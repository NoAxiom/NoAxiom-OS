use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{intrinsics::unlikely, panic, usize};

use async_trait::async_trait;
use downcast_rs::DowncastSync;
use ksync::mutex::{SpinLock, SpinLockGuard};

use crate::{
    constant::fs::{F_OK, R_OK, UID_ROOT, W_OK, X_OK},
    fs::{
        path,
        pipe::PipeDentry,
        vfs::{
            basic::inode::EmptyInode,
            impls::devfs::{
                loop_control::{dentry::LoopControlDentry, inode::LoopControlInode},
                loopdev::{dentry::LoopDevDentry, inode::LoopDevInode},
                null::NullDentry,
                rtc::{dentry::RtcDentry, inode::RtcInode},
                tty::dentry::TtyDentry,
                urandom::dentry::UrandomDentry,
                zero::{dentry::ZeroDentry, inode::ZeroInode},
            },
            root_dentry,
        },
    },
    include::fs::ALL_PERMISSIONS_MASK,
    task::Task,
};

type Mutex<T> = SpinLock<T>;
type MutexGuard<'a, T> = SpinLockGuard<'a, T>;

use super::{
    file::File,
    inode::Inode,
    superblock::{EmptySuperBlock, SuperBlock},
};
use crate::{
    fs::{
        pipe::PipeInode,
        vfs::impls::devfs::{null::NullInode, tty::inode::TtyInode, urandom::inode::UrandomInode},
    },
    include::{
        fs::{DevT, FileFlags, InodeMode},
        result::Errno,
    },
    sched::utils::block_on,
    syscall::SysResult,
};

pub struct DentryMeta {
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
    /// todo: delete the Mutex
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
// todo: Arc can be &Arc to reduce the Arc clone?
pub trait Dentry: Send + Sync + DowncastSync {
    /// Get the meta of the dentry
    fn meta(&self) -> &DentryMeta;
    /// Open the file associated with the dentry
    fn open(self: Arc<Self>, file_flags: &FileFlags) -> SysResult<Arc<dyn File>>;
    /// Get new dentry from name
    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry>;
    /// Create a new dentry with `name` and `mode`
    async fn create(self: Arc<Self>, name: &str, mode: InodeMode) -> SysResult<Arc<dyn Dentry>>;
    /// Create a sym link to `tar_name` in the dentry
    async fn symlink(self: Arc<Self>, name: &str, tar_name: &str) -> SysResult<()>;
    /// Into dyn
    fn into_dyn(self: &Arc<Self>) -> Arc<dyn Dentry>
    where
        Self: Sized,
    {
        self.clone()
    }
}

impl dyn Dentry {
    /// Get the name of the dentry
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.meta().name
    }
    /// Get the inode of the dentry
    pub fn inode(&self) -> SysResult<Arc<dyn Inode>> {
        if let Some(inode) = self.meta().inode.lock().as_ref() {
            Ok(inode.clone())
        } else {
            warn!("[kernel] [dentry] {} inode not exist", self.name());
            Err(Errno::ENOENT)
        }
    }
    /// Set the inode of the dentry
    pub fn set_inode(&self, inode: Arc<dyn Inode>) {
        // if self.meta().inode.lock().is_some() {
        //     assert_eq!(
        //         inode.file_type(),
        //         self.inode().unwrap().file_type(),
        //         "{}",
        //         format!(
        //             "replace inode in {}, type: {:?}",
        //             self.name(),
        //             self.inode().unwrap().file_type()
        //         )
        //     );
        //     return;
        // }
        *self.meta().inode.lock() = Some(inode);
    }
    /// Check if the dentry is negative
    #[inline(always)]
    pub fn is_negative(&self) -> bool {
        self.inode().is_err()
    }

    /// Get the path of the dentry, panic if the dentry is deleted.
    pub fn path(self: &Arc<Self>) -> String {
        let mut path = self.name().to_string();
        let mut current = self.clone();
        while let Some(parent) = current.parent() {
            path = format!("{}/{}", parent.name(), path);
            current = parent;
        }
        if path.len() > 1 {
            path.remove(0);
        }
        debug_assert!(path.starts_with('/'));
        path
    }

    /// Get the parent of the dentry
    /// todo: use Arc directly, not Weak
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

    /// Get super block of the dentry
    pub fn super_block(&self) -> Arc<dyn SuperBlock> {
        self.meta().super_block.clone()
    }

    /// use self.create() to generate child dentry
    /// Add a child dentry with `name` and `child_inode`, for realfs only.
    /// todo: delete this method
    pub fn add_child_with_inode(
        self: &Arc<Self>,
        name: &str,
        child_inode: Arc<dyn Inode>,
    ) -> Arc<dyn Dentry> {
        let mut children = self.meta().children.lock();

        let res = if let Some(child) = children.get(name) {
            trace!("[add_child] {} already exists, pass", name);
            // child.set_inode(child_inode);
            child.clone()
        } else {
            let child = self.clone().from_name(name);
            child.set_inode(child_inode);
            children.insert(name.to_string(), child.clone());
            child
        };
        res
    }

    /// use child dentry directly
    /// Add a child dentry with `child` directly, for fs which doesn't
    /// support create or used in different type fs (like mount).
    pub fn add_child(self: &Arc<Self>, child: Arc<dyn Dentry>) {
        let mut children = self.meta().children.lock();
        let name = child.name().to_string();
        if let Some(old) = children.insert(name.clone(), child) {
            warn!(
                "add child {} to {} already has child {}, replace it",
                name,
                self.name(),
                old.name()
            );
        } else {
            debug!("[dentry] add child {} to {} success", name, self.name());
        }
    }

    /// Remove a child dentry with `name`.
    /// Mention: this will remove the dentry from the path cache,
    /// but not remove the dentry in the real fs!
    pub fn remove_child(self: &Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        let path = self.clone().path();
        crate::fs::path::PATH_CACHE.lock().remove(&path);
        self.children().remove(name)
    }

    /// walk through the path, return ENOENT when not found.
    /// follow the symlink at a limited time
    /// use recursion
    /// todo: add path_cache!!
    pub fn walk_path(
        self: &Arc<Self>,
        task: &Arc<Task>,
        path: &Vec<&str>,
    ) -> SysResult<Arc<dyn Dentry>> {
        Ok(self.__walk_path(Some(task), path, 0, 0)?.0)
    }

    pub fn walk_path_no_checksearch(
        self: &Arc<Self>,
        path: &Vec<&str>,
    ) -> SysResult<Arc<dyn Dentry>> {
        Ok(self.__walk_path(None, path, 0, 0)?.0)
    }

    /// Must ensure the inode has symlink
    /// symlink jump, follow the symlink path
    pub fn symlink_jump(
        self: &Arc<Self>,
        task: &Arc<Task>,
        symlink_path: &str,
    ) -> SysResult<Arc<dyn Dentry>> {
        debug_assert!(self
            .inode()
            .expect("should have inode!")
            .symlink()
            .is_some());
        Ok(self.__symlink_jump(Some(task), symlink_path, 0)?.0)
    }

    fn __symlink_jump(
        self: &Arc<Self>,
        task: Option<&Arc<Task>>,
        symlink_path: &str,
        jumps: usize,
    ) -> SysResult<(Arc<dyn Dentry>, usize)> {
        let abs = symlink_path.starts_with('/');
        let components = path::resolve_path(symlink_path)?;
        let res = if abs {
            root_dentry().__walk_path(task, &components, 0, jumps)
        } else {
            self.parent()
                .expect("must have parent")
                .__walk_path(task, &components, 0, jumps)
        };
        match res {
            Ok((dentry, new_jumps)) => {
                if let Ok(inode) = dentry.inode() {
                    if let Some(symlink_path) = inode.symlink() {
                        debug!("[__symlink_jump] Following symlink: {}", symlink_path);
                        return dentry.__symlink_jump(task, &symlink_path, jumps + 1);
                    }
                }
                Ok((dentry, new_jumps))
            }
            Err(e) => Err(e),
        }
    }

    fn __walk_path(
        self: &Arc<Self>,
        task: Option<&Arc<Task>>,
        path: &Vec<&str>,
        step: usize,
        jumps: usize,
    ) -> SysResult<(Arc<dyn Dentry>, usize)> {
        const SYMLINK_MAX_STEP: usize = 12;
        if unlikely(jumps >= SYMLINK_MAX_STEP) {
            error!("[walk_path] symlink too many times, jumps: {}", jumps);
            return Err(Errno::ELOOP);
        }
        if step == path.len() {
            return Ok((self.clone(), jumps));
        }
        if unlikely(self.is_negative()) {
            error!("[walk_path] {} is negative", self.name());
            return Err(Errno::ENOENT);
        }

        let inode = self.inode().expect("should have inode!");

        let entry = path[step];
        match entry {
            "." => self.__walk_path(task, path, step + 1, jumps),
            ".." => {
                if let Some(parent) = self.parent() {
                    parent.__walk_path(task, path, step + 1, jumps)
                } else {
                    error!("[walk_path] {} has no parent", self.name());
                    Err(Errno::ENOENT)
                }
            }
            name => {
                if let Some(symlink_path) = inode.symlink() {
                    let (tar, new_jumps) = self.__symlink_jump(task, &symlink_path, jumps + 1)?;
                    return tar.__walk_path(task, path, step, new_jumps);
                }
                // Check if this is a directory BEFORE checking permissions
                // This ensures ENOTDIR takes precedence over EACCES
                if inode.file_type() != InodeMode::DIR {
                    error!("[walk_path] {} is not a dir", self.name());
                    return Err(Errno::ENOTDIR);
                }

                // Only check search permissions after confirming it's a directory
                if let Some(task) = task {
                    if task.user_id().fsuid() != 0 {
                        if unlikely(!self.can_search(task)) {
                            error!("[walk_path] has no search access");
                            return Err(Errno::EACCES);
                        }
                    }
                }
                if let Some(child) = self.get_child(name) {
                    return child.__walk_path(task, path, step + 1, jumps);
                }
                debug!(
                    "[walk_path] {} not found in {}, step: {}",
                    name,
                    self.name(),
                    step
                );
                match self.clone().open(&FileFlags::empty()) {
                    Ok(file) => {
                        assert_no_lock!();
                        // todo: add sync fn load_dir method
                        warn!(
                            "[walk_path] {} cannot find {}! Now load this dir.",
                            name,
                            self.name()
                        );
                        block_on(file.load_dir()).expect("can not load dir!");
                    }
                    Err(e) => {
                        error!(
                            "[walk_path] {} open failed with {:?}, not found!",
                            e,
                            self.name()
                        );
                        return Err(Errno::ENOENT);
                    }
                }
                if let Some(child) = self.get_child(name) {
                    return child.__walk_path(task, path, step + 1, jumps);
                }
                #[cfg(feature = "debug_sig")]
                {
                    match name {
                        "localtime" => {}
                        "riscv64-linux-gnu" => {}
                        "ld.so.preload" | "ld.so.cache" => {}
                        "usr" | "tls" | "smaps" | "tmp" => {}
                        "var" | "sys" | "stat" => {}
                        "iozone.tmp.DUMMY" | "iozone.DUMMY" | "iozone.DUMMY.0"
                        | "iozone.DUMMY.1" | "iozone.DUMMY.2" | "iozone.DUMMY.3" => {}
                        "busybox.conf" => {}
                        "oom_score_adj" => {}
                        "mkfs.ext3" | "mkfs.ext4" | "mkfs.xfs" | "mkfs.btrfs" | "mkfs.bcachefs"
                        | "[" => {}
                        "X.1" | "X.4" | "X.7" => {}
                        other => {
                            error!(
                                "[walk_path] {} not found in {}, step: {}",
                                other,
                                self.name(),
                                step
                            );
                        }
                    }
                }
                #[cfg(not(feature = "debug_sig"))]
                {
                    warn!(
                        "[walk_path] {} not found in {}, step: {}",
                        name,
                        self.name(),
                        step
                    );
                }
                Err(Errno::ENOENT)
            }
        }
    }

    /// Hard link, create a son and link it to `target`
    /// todo: real fs support
    pub async fn create_link(
        self: &Arc<Self>,
        target: Arc<dyn Dentry>,
        name: &str,
    ) -> SysResult<isize> {
        debug_assert!(!target.is_negative());
        if self.inode()?.file_type() != InodeMode::DIR {
            error!("[Vfs::link] {} is not a dir", self.name());
            return Err(Errno::ENOTDIR);
        }

        if let Some(old) = self.get_child(name) {
            error!(
                "[Vfs::link] {} already exists in {}",
                old.name(),
                self.name()
            );
            let inode = target.inode()?;
            inode.meta().inner.lock().nlink;
            inode.meta().inner.lock().nlink += 1;
            old.set_inode(inode);
            return Err(Errno::EEXIST);
        }

        let inode = target.inode()?;
        let son = self.clone().from_name(name);
        son.set_inode(inode.clone());
        debug!("[Vfs::linkto] set_inode {} to {}", name, target.name());

        let nlink = inode.meta().inner.lock().nlink;
        inode.meta().inner.lock().nlink += 1;

        self.add_child(son);

        Ok(nlink as isize)
    }

    /// Symlink, create a son and set its inode's symlink to `target`
    /// todo: real fs support
    pub async fn create_symlink(self: &Arc<Self>, target: String, name: &str) -> SysResult<()> {
        if self.inode()?.file_type() != InodeMode::DIR {
            error!("[Vfs::symlink] {} is not a dir", self.name());
            return Err(Errno::ENOTDIR);
        }

        // todo: check existed

        let son = self
            .clone()
            .create(
                name,
                InodeMode::LINK | InodeMode::from_bits(ALL_PERMISSIONS_MASK).unwrap(),
            )
            .await?;
        son.inode()?.set_symlink(target.clone());
        debug!("[Vfs::symlink] set_symlink {} to {}", target, name);

        Ok(())
    }

    /// Check if the dentry has access to the file.
    pub fn check_arrive(self: &Arc<Self>) -> SysResult<()> {
        if self.inode()?.privilege().bits() & 0o111 == 0 {
            warn!(
                "[check_arrive] {} has no access, its access is {}",
                self.name(),
                self.inode()?.privilege().bits()
            );
            return Err(Errno::EACCES);
        }
        if let Some(parent) = self.parent() {
            return parent.check_arrive();
        }
        Ok(())
    }

    pub fn check_access(
        self: &Arc<Self>,
        task: &Arc<Task>,
        access: i32,
        is_fs: bool,
    ) -> SysResult<()> {
        let user_id = task.user_id();
        let (uid, gid) = if is_fs {
            (user_id.fsuid(), user_id.fsgid())
        } else {
            (user_id.uid(), user_id.gid())
        };

        let dentry = self;
        let inode = self.inode()?;
        let pri = inode.privilege();

        if uid == UID_ROOT {
            if (access & X_OK != 0) && (pri.bits() & 0o111 == 0) {
                error!(
                    "[sys_faccessat] root user cannot execute file: {:?}",
                    self.path()
                );
                return Err(Errno::EACCES);
            }
            return Ok(());
        }

        debug!(
            "[check_access] uid: {}, gid: {}, inode uid: {}, inode gid: {}",
            uid,
            gid,
            inode.uid(),
            inode.gid()
        );

        // check if the parent directory is accessible
        if let Some(parent) = dentry.parent() {
            parent.check_arrive()?;
        }

        if access == F_OK {
            return Ok(());
        }

        let permission = if uid == inode.uid() {
            pri.user_permissions() as i32
        } else if gid == inode.gid() {
            pri.group_permissions() as i32
        } else {
            pri.other_permissions() as i32
        };

        if (access & X_OK != 0) && (permission & X_OK == 0) {
            error!("[check_access] user cannot execute file: {:?}", self.path());
            return Err(Errno::EACCES);
        }

        if (access & R_OK != 0) && (permission & R_OK == 0) {
            error!("[check_access] user cannot read file: {:?}", self.path());
            return Err(Errno::EACCES);
        }

        if (access & W_OK != 0) && (permission & W_OK == 0) {
            error!("[check_access] user cannot write file: {:?}", self.path());
            return Err(Errno::EACCES);
        }
        Ok(())
    }

    pub fn can_search(self: &Arc<Self>, task: &Arc<Task>) -> bool {
        let user_id = task.user_id();
        let (euid, egid) = { (user_id.fsuid(), user_id.fsgid()) };
        if euid == 0 {
            return true;
        }
        let i_mode = self.inode().unwrap().inode_mode().bits();
        debug!(
            "[can_search] Checking search permission for {}, mode: {:o}, euid: {}, egid: {}",
            self.path(),
            i_mode,
            euid,
            egid
        );
        let (user_perm, group_perm, other_perm) =
            ((i_mode >> 6) & 0o7, (i_mode >> 3) & 0o7, i_mode & 0o7);
        let perm = if euid == self.inode().unwrap().uid() {
            user_perm
        } else if egid == self.inode().unwrap().gid() {
            group_perm
        } else {
            other_perm
        };
        if perm & 0o111 == 0 {
            error!(
                "[can_search] No search permission for {}, mode: {:o}, euid: {}, egid: {}",
                self.path(),
                i_mode,
                euid,
                egid
            );
            return false;
        }
        true
    }

    // todo: improve this
    pub fn mknodat_son(
        self: &Arc<Self>,
        name: &str,
        dev_t: DevT,
        mode: InodeMode,
    ) -> SysResult<()> {
        let file_type = InodeMode::file_type(mode.bits())?;
        let super_block = self.super_block();
        match file_type {
            InodeMode::FIFO => {
                let pipe_dentry = Arc::new(PipeDentry::new(name)).into_dyn();
                let pipe_inode = Arc::new(PipeInode::new());
                pipe_dentry.set_inode(pipe_inode);
                pipe_dentry.inode().unwrap().set_inode_mode(mode);
                self.add_child(pipe_dentry); // not set parent correctly!
                root_dentry().remove_child(name);
                Ok(())
            }
            InodeMode::CHAR => {
                match dev_t.new_decode_dev() {
                    (1, 3) => {
                        // /dev/null
                        let son_dentry = Arc::new(NullDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(NullInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (1, 5) => {
                        // /dev/zero
                        let son_dentry = Arc::new(ZeroDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(ZeroInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (1, 9) => {
                        // /dev/urandom
                        let son_dentry = Arc::new(UrandomDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(UrandomInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (5, 0) => {
                        // /dev/tty
                        let son_dentry = Arc::new(TtyDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(TtyInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (10, 0) => {
                        // /dev/rtc
                        let son_dentry = Arc::new(RtcDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(RtcInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (10, 237) => {
                        // /dev/loop-control
                        let son_dentry = Arc::new(LoopControlDentry::new(
                            Some(self.clone()),
                            name,
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(LoopControlInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        Ok(())
                    }
                    (x, y) => {
                        error!(
                            "[mknodat] Unsupported char device: major {}, minor {}",
                            x, y
                        );
                        return Err(Errno::ENOSYS);
                    }
                }
            }
            InodeMode::BLOCK => {
                match dev_t.new_decode_dev() {
                    (7, y) => {
                        // /dev/loop{y}
                        let son_dentry = Arc::new(LoopDevDentry::new(
                            Some(self.clone()),
                            &format!("loop{}", y),
                            super_block.clone(),
                        ))
                        .into_dyn();
                        let son_inode = Arc::new(LoopDevInode::new(super_block));
                        son_dentry.set_inode(son_inode);
                        son_dentry.inode().unwrap().set_inode_mode(mode);
                        self.add_child(son_dentry);
                        debug!("[mknodat] create loop device: loop{}", y);
                        Ok(())
                    }
                    (x, y) => {
                        error!(
                            "[mknodat] Unsupported char device: major {}, minor {}",
                            x, y
                        );
                        Err(Errno::ENOSYS)
                    }
                }
            }
            mode => {
                error!("[mknodat] Unsupported inode type {:?} for mknodat", mode);
                Err(Errno::EINVAL)
            }
        }
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

    fn open(self: Arc<Self>, _file_flags: &FileFlags) -> SysResult<Arc<dyn File>> {
        unreachable!()
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!()
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!()
    }

    async fn symlink(self: Arc<Self>, _name: &str, _tar_name: &str) -> SysResult<()> {
        unreachable!()
    }
}

lazy_static::lazy_static! {
    pub static ref DENTRY_HERE: Arc<dyn Dentry> = {
        let ret = Arc::new(EmptyDentry::new("."));
        ret.into_dyn().set_inode(Arc::new(EmptyInode::new()));
        ret.into_dyn().inode().unwrap().set_inode_mode(InodeMode::DIR | InodeMode::from_bits(0o755).unwrap());
        ret
    };
    pub static ref DENTRY_FRONT: Arc<dyn Dentry> = {
        let ret = Arc::new(EmptyDentry::new(".."));
        ret.into_dyn().set_inode(Arc::new(EmptyInode::new()));
        ret.into_dyn().inode().unwrap().set_inode_mode(InodeMode::DIR | InodeMode::from_bits(0o755).unwrap());
        ret
    };
}

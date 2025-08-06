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
    fs::{
        path,
        vfs::{
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
        vfs::{
            basic::inode::InodeState,
            impls::devfs::{null::NullInode, tty::inode::TtyInode, urandom::inode::UrandomInode},
        },
    },
    include::{
        fs::{DevT, FileFlags, InodeMode, RenameFlags},
        result::Errno,
    },
    sched::utils::block_on,
    syscall::{SysResult, SyscallResult},
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
    pub fn set_inode_none(&self) {
        *self.meta().inode.lock() = None;
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
            warn!("[add_child] {} already exists, replace it", name);
            child.set_inode(child_inode);
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
        }
    }

    /// Remove a child dentry with `name`.
    pub fn remove_child(self: &Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        let path = self.clone().path();
        crate::fs::path::PATH_CACHE.lock().remove(&path);
        self.children().remove(name)
    }

    /// walk through the path, return ENOENT when not found.
    /// follow the symlink at a limited time
    /// use recursion
    /// todo: add path_cache!!
    pub fn walk_path(self: &Arc<Self>, path: &Vec<&str>) -> SysResult<Arc<dyn Dentry>> {
        Ok(self.__walk_path(path, 0, 0)?.0)
    }

    /// Must ensure the inode has symlink
    /// symlink jump, follow the symlink path
    pub fn symlink_jump(self: &Arc<Self>, symlink_path: &str) -> SysResult<Arc<dyn Dentry>> {
        debug_assert!(self
            .inode()
            .expect("should have inode!")
            .symlink()
            .is_some());
        Ok(self.__symlink_jump(symlink_path, 0)?.0)
    }

    fn __symlink_jump(
        self: &Arc<Self>,
        symlink_path: &str,
        jumps: usize,
    ) -> SysResult<(Arc<dyn Dentry>, usize)> {
        let abs = symlink_path.starts_with('/');
        let components = path::resolve_path(symlink_path)?;
        if abs {
            root_dentry().__walk_path(&components, 0, jumps)
        } else {
            self.__walk_path(&components, 0, jumps)
        }
    }

    fn __walk_path(
        self: &Arc<Self>,
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
        // the mid dentry MUST have a inode
        if inode.file_type() != InodeMode::DIR {
            error!("[walk_path] {} is not a dir", self.name());
            return Err(Errno::ENOTDIR);
        }

        let entry = path[step];
        match entry {
            "." => self.__walk_path(path, step + 1, jumps),
            ".." => {
                if let Some(parent) = self.parent() {
                    parent.__walk_path(path, step + 1, jumps)
                } else {
                    error!("[walk_path] {} has no parent", self.name());
                    Err(Errno::ENOENT)
                }
            }
            name => {
                if let Some(symlink_path) = inode.symlink() {
                    let (tar, new_jumps) = self.__symlink_jump(&symlink_path, jumps + 1)?;
                    return tar.__walk_path(path, step + 1, new_jumps);
                }
                if let Some(child) = self.get_child(name) {
                    return child.__walk_path(path, step + 1, jumps);
                }
                match self.clone().open(&FileFlags::empty()) {
                    Ok(file) => {
                        assert_no_lock!();
                        // todo: add sync fn load_dir method
                        warn!("[walk_path] {} is not open! Now open it.", self.name());
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
                    return child.__walk_path(path, step + 1, jumps);
                }
                error!(
                    "[walk_path] {} not found in {}, step: {}",
                    name,
                    self.name(),
                    step
                );
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

        let son = self
            .clone()
            .create(
                name,
                InodeMode::LINK | InodeMode::from_bits(ALL_PERMISSIONS_MASK).unwrap(),
            )
            .await?;
        son.inode()?.set_symlink(target.clone());
        debug!("[Vfs::symlink] set_symlink {} to {}", name, target);

        Ok(())
    }

    /// Unlink, unlink self and delete the inner file if nlink is 0.
    pub async fn unlink(self: Arc<Self>, file_flags: &FileFlags) -> SyscallResult {
        if unlikely(self.name() == "interrupts") {
            // MENTION: this is required by official
            return Err(Errno::ENOSYS);
        }

        let inode = if let Ok(inode) = self.inode() {
            inode
        } else {
            return Ok(0);
        };
        let mut nlink = inode.meta().inner.lock().nlink;
        debug!(
            "[Vfs::unlink] nlink: {}, file_type: {:?}",
            nlink,
            inode.file_type()
        );
        nlink -= 1;
        if nlink == 0 {
            let parent = self.parent().unwrap();
            let mut w_guard = crate::fs::pagecache::get_pagecache_wguard();
            let file = self.clone().open(file_flags)?;
            w_guard.mark_deleted(&file);
            drop(w_guard);
            self.set_inode_none();
            inode.set_state(InodeState::Deleted).await;
            // parent.remove_child(&self.name()).unwrap();
            parent
                .open(&FileFlags::empty())
                .unwrap()
                .delete_child(&self.name())
                .await?;
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
                .open(&FileFlags::empty())?
                .delete_child(tar_name)
                .await?;
        }

        let target = tar_parent.remove_child(target.name()).unwrap();

        // copy
        tar_parent.create(self.name(), self_file_type).await?;

        if flags.contains(RenameFlags::RENAME_EXCHANGE) {
            self.set_inode(target.inode()?);
        } else {
            self.set_inode_none();
        }

        Ok(())
    }

    /// Check if the dentry has access to the file.
    pub fn check_access(self: &Arc<Self>) -> SysResult<()> {
        if self.inode()?.privilege().bits() & 0o111 == 0 {
            warn!(
                "[check_access] {} has no access, its access is {}",
                self.name(),
                self.inode()?.privilege().bits()
            );
            return Err(Errno::EACCES);
        }
        if let Some(parent) = self.parent() {
            return parent.check_access();
        }
        Ok(())
    }

    // todo: improve this
    pub fn mknodat_son(
        self: &Arc<Self>,
        name: &str,
        dev_t: DevT,
        mode: InodeMode,
    ) -> SysResult<()> {
        let inode = self.inode()?;
        let file_type = inode.file_type();
        let super_block = self.super_block();
        match file_type {
            InodeMode::FIFO => {
                self.set_inode(Arc::new(PipeInode::new()));
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
            _ => {
                error!("[mknodat] Unsupported inode type for mknodat");
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

use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use crate::{
    fs::vfs::{
        basic::{
            dentry::{Dentry, DentryMeta},
            file::File,
            superblock::SuperBlock,
        },
        impls::devfs::loop_control::file::get_loop_control,
    },
    include::{
        fs::{FileFlags, InodeMode},
        result::Errno,
    },
    syscall::SysResult,
};

pub struct LoopDevDentry {
    meta: DentryMeta,
}

impl LoopDevDentry {
    pub fn new(
        parent: Option<Arc<dyn Dentry>>,
        name: &str,
        super_block: Arc<dyn SuperBlock>,
    ) -> Self {
        Self {
            meta: DentryMeta::new(parent, name, super_block),
        }
    }
}

#[async_trait]
impl Dentry for LoopDevDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let super_block = self.meta.super_block.clone();
        Arc::new(Self::new(Some(self), name, super_block))
    }

    fn open(self: Arc<Self>, _file_flags: &FileFlags) -> SysResult<Arc<dyn File>> {
        let dentry = self.into_dyn();
        let name = dentry.name();
        let device_id = name
            .strip_prefix("loop")
            .ok_or(Errno::EINVAL)?
            .parse::<usize>()
            .map_err(|_| Errno::EINVAL)?;
        Ok(get_loop_control().get(device_id).unwrap())
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        Err(Errno::ENOSYS)
    }

    async fn symlink(self: Arc<Self>, _name: &str, _tar_name: &str) -> SysResult<()> {
        unreachable!("LoopDevDentry should not create symlink");
    }
}

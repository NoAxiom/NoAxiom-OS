use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::file::CpuDmaLatencyFile;
use crate::{
    fs::vfs::basic::{
        dentry::{Dentry, DentryMeta},
        file::{File, FileMeta},
        superblock::SuperBlock,
    },
    include::fs::InodeMode,
    syscall::SysResult,
};

pub struct CpuDmaLatencyDentry {
    meta: DentryMeta,
}

impl CpuDmaLatencyDentry {
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
impl Dentry for CpuDmaLatencyDentry {
    fn meta(&self) -> &DentryMeta {
        &self.meta
    }

    fn from_name(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unreachable!("CpuDmaLatency dentry should not have child");
    }

    fn open(self: Arc<Self>) -> SysResult<Arc<dyn File>> {
        Ok(Arc::new(CpuDmaLatencyFile::new(FileMeta::new(
            self.clone(),
            self.inode()?,
        ))))
    }

    async fn create(self: Arc<Self>, _name: &str, _mode: InodeMode) -> SysResult<Arc<dyn Dentry>> {
        unreachable!("CpuDmaLatency dentry should not create child");
    }
}

use crate::fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta};

pub struct ProcDevFsSuperBlock {
    meta: SuperBlockMeta,
}

impl ProcDevFsSuperBlock {
    pub fn new(meta: SuperBlockMeta) -> Self {
        Self { meta }
    }
}

impl SuperBlock for ProcDevFsSuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}

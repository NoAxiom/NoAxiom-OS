use crate::fs::vfs::basic::superblock::{SuperBlock, SuperBlockMeta};

pub struct DevFsSuperBlock {
    meta: SuperBlockMeta,
}

impl DevFsSuperBlock {
    pub fn new(meta: SuperBlockMeta) -> Self {
        Self { meta }
    }
}

impl SuperBlock for DevFsSuperBlock {
    fn meta(&self) -> &SuperBlockMeta {
        &self.meta
    }
}

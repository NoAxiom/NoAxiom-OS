use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;

use super::file::NullFile;
use crate::{
    dentry_default,
    fs::vfs::basic::{
        dentry::{Dentry, DentryMeta},
        file::{File, FileMeta},
        superblock::SuperBlock,
    },
    include::fs::InodeMode,
    syscall::SysResult,
};

dentry_default!(NullDentry, NullFile);

use crate::{dentry_default, fs::vfs::impls::proc::meminfo::file::MemInfoFile};

dentry_default!(MemInfoDentry, MemInfoFile);

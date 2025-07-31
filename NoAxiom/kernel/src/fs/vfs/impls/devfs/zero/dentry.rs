use crate::{dentry_default, fs::vfs::impls::devfs::zero::file::ZeroFile};
dentry_default!(ZeroDentry, ZeroFile);

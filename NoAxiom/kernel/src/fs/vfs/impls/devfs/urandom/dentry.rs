use crate::{dentry_default, fs::vfs::impls::devfs::urandom::file::UrandomFile};
dentry_default!(UrandomDentry, UrandomFile);

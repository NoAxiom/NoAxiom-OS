use crate::{dentry_default, fs::vfs::impls::devfs::tty::file::TtyFile};

dentry_default!(TtyDentry, TtyFile);

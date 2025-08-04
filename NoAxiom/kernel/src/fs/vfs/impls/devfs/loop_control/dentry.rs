use crate::{dentry_default, fs::vfs::impls::devfs::loop_control::file::LoopControlFile};

dentry_default!(LoopControlDentry, LoopControlFile);

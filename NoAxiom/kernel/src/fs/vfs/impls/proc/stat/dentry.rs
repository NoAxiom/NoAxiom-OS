use crate::{dentry_default, fs::vfs::impls::proc::stat::file::ProcStatFile};

dentry_default!(ProcStatDentry, ProcStatFile);

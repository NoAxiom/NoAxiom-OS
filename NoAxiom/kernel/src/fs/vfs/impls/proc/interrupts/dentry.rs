use crate::{dentry_default, fs::vfs::impls::proc::interrupts::file::InterruptsFile};

dentry_default!(InterruptsDentry, InterruptsFile);

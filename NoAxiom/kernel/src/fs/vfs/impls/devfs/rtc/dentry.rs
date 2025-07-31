use crate::{dentry_default, fs::vfs::impls::devfs::rtc::file::RtcFile};

dentry_default!(RtcDentry, RtcFile);

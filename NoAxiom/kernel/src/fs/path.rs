use alloc::string::{String, ToString};

use super::vfs::root_dentry;

#[derive(Debug, Clone)]
pub struct Path {
    inner: String,
}

impl Path {
    pub fn new(inner: String) -> Self {
        Self { inner }
    }
    pub fn as_string(self) -> String {
        self.inner
    }
    pub fn inner(&self) -> &String {
        &self.inner
    }
}

impl From<String> for Path {
    fn from(inner: String) -> Self {
        Self { inner }
    }
}

impl From<&str> for Path {
    fn from(inner: &str) -> Self {
        Self {
            inner: inner.to_string(),
        }
    }
}

pub fn check_path(path: &str) -> bool {
    root_dentry().find(path).is_some()
}

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::fmt::Debug;

use super::vfs::{basic::dentry::Dentry, root_dentry};
use crate::{include::fs::InodeMode, syscall::SysResult};

#[derive(Clone)]
pub struct Path {
    inner: String,
    dentry: Arc<dyn Dentry>,
}

impl Path {
    /// Get the path from absolute path, the path should exist
    pub fn try_from(abs_path: String) -> SysResult<Self> {
        assert!(abs_path.starts_with('/'));
        trace!("Path::from: {}", abs_path);
        let mut split_path = abs_path.split('/').collect::<Vec<&str>>();
        if split_path.ends_with(&[""]) {
            split_path.pop();
        }
        let dentry = root_dentry().find_path(&split_path)?;
        Ok(Self {
            inner: abs_path,
            dentry,
        })
    }

    /// Get the path from absolute path, create the path if not exist
    pub async fn from_or_create(abs_path: String, mode: InodeMode) -> Self {
        assert!(abs_path.starts_with('/'));
        trace!("Path::from_or_create: {}", abs_path);
        let mut split_path = abs_path.split('/').collect::<Vec<&str>>();
        if split_path.ends_with(&[""]) {
            split_path.pop();
        }
        let dentry = root_dentry().find_path_or_create(&split_path, mode).await; // todo: don't walk from root
        Self {
            inner: abs_path,
            dentry,
        }
    }

    fn cd(&self, path: &str) -> String {
        assert!(!path.starts_with('/'));
        let mut new_path = self.inner.to_string();
        if new_path.ends_with('/') {
            new_path.pop();
        }

        let path_parts: Vec<&str> = path.split('/').collect();
        let mut result_parts: Vec<String> = new_path.split('/').map(String::from).collect();

        for part in path_parts {
            match part {
                "" | "." => continue,
                ".." => {
                    if result_parts.len() > 1 {
                        result_parts.pop();
                    } else {
                        error!("Path::from_cd: path underflow");
                    }
                }
                _ => result_parts.push(part.to_string()),
            }
        }

        if result_parts.len() == 1 {
            "/".to_string()
        } else {
            result_parts.join("/")
        }
    }

    /// Get the path from relative path, the path should exist
    pub fn from_cd(&self, path: &str) -> SysResult<Self> {
        Self::try_from(self.cd(path))
    }

    /// Get the path from relative path, create the path if not exist
    pub async fn from_cd_or_create(&self, path: &str, mode: InodeMode) -> Self {
        Self::from_or_create(self.cd(path), mode).await
    }

    /// Get dentry
    pub fn dentry(&self) -> Arc<dyn Dentry> {
        self.dentry.clone()
    }

    pub fn as_string(&self) -> String {
        self.inner.clone()
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

use alloc::{string::String, sync::Arc, vec::Vec};
use core::intrinsics::unlikely;

use hashbrown::HashMap;
use include::errno::Errno;
use kfuture::block::block_on;
use ksync::mutex::SpinLock;

use super::vfs::basic::dentry::Dentry;
use crate::{
    constant::fs::AT_FDCWD,
    fs::vfs::root_dentry,
    include::fs::{InodeMode, SearchFlags},
    syscall::SysResult,
    task::Task,
};

lazy_static::lazy_static! {
    pub static ref PATH_CACHE: SpinLock<HashMap<String, Arc<dyn Dentry>>> = SpinLock::new(HashMap::new());
}

const MAX_NAME_LEN: usize = 255;

/// Open a file by path for kernel use by sync
///
/// return PANIC for not found or any fault
///
/// MENTION:
/// - make sure the file existed
/// - only support Absolute path
/// - only support open the normal file, cannot open the dir
/// - don't support CREATE
/// - don't check the access
/// - can handle the symlink
pub fn kopen(path: &str) -> Arc<dyn Dentry> {
    debug_assert!(path.starts_with('/'), "kopen only support absolute path");
    let paths = resolve_path(path).expect("resolve path failed");
    debug!("[kopen] paths: {:?}", paths);
    root_dentry()
        .walk_path(&paths)
        .expect("kopen failed, please check the path")
}

/// Create a file by path for kernel use by sync
///
/// return PANIC for not found or any fault
///
/// MENTION:
/// - make sure the path throght dir existed
/// - only support Absolute path
/// - support FILE / DIR creation
/// - don't check the access
/// - can handle the symlink
pub fn kcreate(path: &str, mode: InodeMode) -> Arc<dyn Dentry> {
    debug_assert!(path.starts_with('/'), "kcreate only support absolute path");
    let (paths, last) = resolve_path2(path).expect("resolve path failed");
    debug!("[kcreate] paths: {:?}, last: {:?}", paths, last);
    let name = last.expect("kcreate must have a name at the end");
    let parent = root_dentry().walk_path(&paths).expect("walk path failed");
    assert_no_lock!();
    block_on(parent.create(name, mode)).unwrap()
}

/// the async version of kcreate
pub async fn kcreate_async(path: &str, mode: InodeMode) -> Arc<dyn Dentry> {
    debug_assert!(path.starts_with('/'), "kcreate only support absolute path");
    let (paths, last) = resolve_path2(path).expect("resolve path failed");
    debug!("[kcreate] paths: {:?}, last: {:?}", paths, last);
    let name = last.expect("kcreate must have a name at the end");
    let parent = root_dentry().walk_path(&paths).expect("walk path failed");
    assert_no_lock!();
    parent.create(name, mode).await.unwrap()
}

/// split a path string into components
/// just string operation
///
/// return the components as a vector of strings
pub fn resolve_path(path: &str) -> SysResult<Vec<&str>> {
    let paths = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();
    for path in &paths {
        if path.len() > MAX_NAME_LEN {
            return Err(Errno::ENAMETOOLONG);
        }
    }
    Ok(paths)
}

/// split a path string into components
/// just string operation
///
/// return the components as a vector of strings, the last component is special
/// listed
fn resolve_path2<'a>(path: &'a str) -> SysResult<(Vec<&'a str>, Option<&'a str>)> {
    let mut paths = resolve_path(path)?;
    let last = paths.pop();
    Ok((paths, last))
}

/// lookup a file from dirfd and path with AtFlags
///
/// return the dentry if existed or any fault
///
/// Mention:
/// - AtFlags support AT_SYMLINK_NOFOLLOW only currently
/// - ignore the access
pub fn get_dentry(
    task: &Arc<Task>,
    dirfd: isize,
    path: &str,
    flags: &SearchFlags,
) -> SysResult<Arc<dyn Dentry>> {
    let abs = path.starts_with('/');
    let components = resolve_path(path)?;
    debug!(
        "[get_dentry] dirfd: {}, abs: {}, components: {:?}",
        dirfd, abs, components
    );
    __get_dentry(task, dirfd, abs, &components, flags)
}

/// lookup a file from dirfd and path with AtFlags
///
/// return the PARENT dentry and the final NAME if existed or any fault
///
/// Mention:
/// - AtFlags support AT_SYMLINK_NOFOLLOW only currently
/// - ignore the access
pub fn get_dentry_parent<'a>(
    task: &Arc<Task>,
    dirfd: isize,
    path: &'a str,
    flags: &SearchFlags,
) -> SysResult<(Arc<dyn Dentry>, &'a str)> {
    let abs = path.starts_with('/');
    let (components, last) = resolve_path2(path)?;
    debug!(
        "[get_dentry_parent] dirfd: {}, abs: {}, components: {:?}, last: {:?}",
        dirfd, abs, components, last
    );
    if unlikely(last.is_none()) {
        return Err(Errno::EEXIST);
    }
    debug_assert!(
        last.is_some(),
        "get_dentry_parent must have a name at the end"
    );
    Ok((
        __get_dentry(task, dirfd, abs, &components, flags)?,
        last.unwrap(),
    ))
}

fn __get_dentry(
    task: &Arc<Task>,
    dirfd: isize,
    abs: bool,
    components: &Vec<&str>,
    flags: &SearchFlags,
) -> SysResult<Arc<dyn Dentry>> {
    let cwd = if !abs {
        // relative path
        if dirfd == AT_FDCWD {
            task.cwd().clone()
        } else {
            task.fd_table()
                .get(dirfd as usize)
                .ok_or(Errno::EBADF)?
                .dentry()
        }
    } else {
        // absolute path
        task.root().clone()
    };

    let mut this = cwd.walk_path(components)?;

    // special case for the last component
    // if the last component is a symlink, we need to follow it
    if let Ok(inode) = this.inode() {
        if !flags.contains(SearchFlags::AT_SYMLINK_NOFOLLOW) {
            if let Some(symlink_path) = inode.symlink() {
                debug!("[get_dentry] Following symlink: {}", symlink_path);
                this = this.symlink_jump(&symlink_path)?;
            } else {
                trace!(
                    "[get_dentry] AT_SYMLINK_NOFOLLOW is not set, but Dentry {} has no symlink",
                    this.name()
                );
            }
        } else {
            debug!("[get_dentry] AT_SYMLINK_NOFOLLOW is set, not following symlink");
        }
    } else {
        warn!("[get_dentry] Son Dentry is a negative dentry!");
    }

    Ok(this)
}

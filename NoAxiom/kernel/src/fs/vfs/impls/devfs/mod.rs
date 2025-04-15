use alloc::sync::Arc;

use tty::{dentry::TtyDentry, inode::TtyInode};

use crate::{fs::vfs::basic::dentry::Dentry, syscall::SysResult};

pub mod filesystem;
mod null;
mod rtc;
mod superblock;
mod tty;
mod zero;

pub async fn init(fs_root: Arc<dyn Dentry>) -> SysResult<()> {
    assert_eq!(fs_root.name(), "dev");

    info!("[fs] [proc] create /dev/tty");
    let tty_dentry = TtyDentry::new(Some(fs_root.clone()), "tty", fs_root.super_block());
    let tty_inode = Arc::new(TtyInode::new(fs_root.super_block()));
    tty_dentry.set_inode(tty_inode);
    fs_root.add_child_directly(Arc::new(tty_dentry));

    Ok(())
}

use alloc::sync::Arc;

use ksync::Once;
use tty::{dentry::TtyDentry, inode::TtyInode};

use crate::{
    fs::vfs::basic::{dentry::Dentry, file::File},
    include::fs::FileFlags,
    syscall::SysResult,
};

pub mod filesystem;
mod null;
mod rtc;
mod superblock;
mod tty;
mod zero;

pub static TTYFILE: Once<Arc<dyn File>> = Once::new();

pub async fn init(fs_root: Arc<dyn Dentry>) -> SysResult<()> {
    assert_eq!(fs_root.name(), "dev");

    info!("[fs] [proc] create /dev/tty");
    let tty_dentry = TtyDentry::new(Some(fs_root.clone()), "tty", fs_root.super_block());
    let tty_inode = Arc::new(TtyInode::new(fs_root.super_block()));
    tty_dentry.set_inode(tty_inode);
    let tty_dentry: Arc<dyn Dentry> = Arc::new(tty_dentry);
    let tty_file = tty_dentry.clone().open()?;
    tty_file.set_flags(FileFlags::O_RDWR);
    TTYFILE.call_once(|| tty_file);
    fs_root.add_child_directly(tty_dentry);

    Ok(())
}

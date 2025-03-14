use alloc::sync::Arc;

use ext4_rs::{Errno as ext4Errno, Ext4Error};

use super::disk_cursor::DiskCursor;
use crate::{
    device::block::BlockDevice,
    fs::blockcache::AsyncBlockCache,
    include::{fs::Ext4DirEntryType, result::Errno},
    sched::utils::block_on,
};

pub mod dentry;
mod disk_cursor;
pub mod file;
pub mod filesystem;
pub mod inode;
pub mod superblock;

type IExtFs = ext4_rs::Ext4;
type IExtInode = ext4_rs::ext4_defs::Ext4InodeRef;

pub const fn fs_err(err: Ext4Error) -> Errno {
    let err_value = err.error();
    match err_value {
        ext4Errno::ENOENT => Errno::ENOENT,
        ext4Errno::EIO => Errno::EIO,
        ext4Errno::ENOSPC => Errno::ENOSPC,
        ext4Errno::ENOTDIR => Errno::ENOTDIR,
        ext4Errno::EISDIR => Errno::EISDIR,
        ext4Errno::EINVAL => Errno::EINVAL,
        // todo: add more error mapping
        _ => Errno::EIO,
    }
}

pub async fn ext4_rs_test(device: Arc<dyn BlockDevice>) {
    let blk = Arc::new(AsyncBlockCache::from(device));
    let disk_cursor = DiskCursor::new(blk.clone(), 0, 0);
    let ext4 = IExtFs::open(Arc::new(disk_cursor)).await;

    const ROOT_INODE: u32 = 2;
    let inode = block_on(ext4.get_inode_ref(ROOT_INODE));

    debug!(
        "inode {}, file_type: {:?}",
        inode.inode_num,
        inode.inode.file_type()
    );

    // dir ls
    let entries = block_on(ext4.dir_get_entries(ROOT_INODE));
    log::info!("dir ls root");
    for entry in entries.clone() {
        log::info!("{:?}", entry.get_name());
    }

    let self_path = "/";
    for entry in entries {
        let child_name = entry.get_name();
        if child_name == "." || child_name == ".." {
            debug!("load_dir: {:?}, passed", child_name);
            continue;
        }
        let child_path = if self_path != "/" {
            format!("{}/{}", self_path, entry.get_name())
        } else {
            format!("/{}", entry.get_name())
        };
        if entry.get_de_type() == Ext4DirEntryType::EXT4_DE_DIR.bits() {
            let inode_num = block_on(ext4.ext4_dir_open(&child_path)).unwrap();
            let inode = block_on(ext4.get_inode_ref(inode_num));
            debug!("load_dir [{inode_num}] {child_path}: {:?}", child_name);
        } else if entry.get_de_type() == Ext4DirEntryType::EXT4_DE_REG_FILE.bits() {
            let inode_num = block_on(ext4.ext4_file_open(&child_path, "r+")).unwrap();
            let inode = block_on(ext4.get_inode_ref(inode_num));
            debug!("load_file [{inode_num}] {child_path}: {:?}", child_name);
        } else {
            unreachable!();
        };
    }

    let child_path = "/test_chdirA";
    let inode_num = block_on(ext4.ext4_file_open(&child_path, &"w+")).unwrap();
    debug!("OK inode_num: {}", inode_num);

    let child_path = "/test_chdirB";
    block_on(ext4.dir_mk(&child_path)).unwrap();
    let inode_num = block_on(ext4.ext4_dir_open(&child_path)).unwrap();

    let entries = block_on(ext4.dir_get_entries(ROOT_INODE));
    log::info!("dir ls root here");
    for entry in entries.clone() {
        log::info!("{:?}", entry.get_name());
    }
}

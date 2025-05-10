use ext4_rs::{Errno as ext4Errno, Ext4Error};

use crate::{fs::vfs::impls::disk_cursor::DiskCursor, include::result::Errno};

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

use alloc::sync::Arc;

use driver::devices::impls::block::BlockDevice;
#[allow(dead_code, unused_variables)]
pub fn ext4_rs_test(device: Arc<&'static dyn BlockDevice>) {
    use ext4_rs::ext4_defs::*;

    use crate::sched::utils::block_on;

    let disk = Arc::new(DiskCursor::new(device, 0, 0));
    let ext4 = block_on(Ext4::open(disk));

    log::debug!("test write");
    /*
    The maximum size of the `write` operation:

    loongarch64: 0x4000  (16384 or 32 BLOCKS)
    riscv64:     0x10000 (40960 or 80 BLOCKS)
     */

    log::info!("----create file----");
    let inode_mode = InodeFileType::S_IFREG.bits();
    let inode_perm = (InodePerm::S_IREAD | InodePerm::S_IWRITE).bits();
    let inode_ref =
        block_on(ext4.create(ROOT_INODE, "JUSTTEST.txt", inode_mode | inode_perm)).unwrap();
    log::info!("----write file----");
    let test_offset = 0x4000;
    let test_size = 0x100;
    let test_buf_size = 0x100000;
    const BLOCK_SIZE: usize = 512;
    let write_buf = vec![0x41 as u8; test_buf_size];
    let r = block_on(ext4.write_at(inode_ref.inode_num, 0, &write_buf));
    // let buf = vec![0u8; test_size];
    // let r = block_on(ext4.read_at(inode_ref.inode_num, test_offset, &mut buf));
    let mut all = vec![0u8; test_buf_size];
    let r = block_on(ext4.read_at(inode_ref.inode_num, 0, &mut all));

    for i in 0..test_buf_size {
        if all[i] != write_buf[i] {
            panic!(
                "write and read failed at {}: {} != {}",
                i, all[i], write_buf[i]
            );
        }
    }
}

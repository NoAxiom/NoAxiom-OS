use alloc::sync::Arc;

use exe::{dentry::ExeDentry, inode::ExeInode};
use meminfo::{dentry::MemInfoDentry, inode::MemInfoInode};
use mounts::dentry::MountsDentry;
use status::{dentry::StatusDentry, inode::StatusInode};

use crate::{
    fs::vfs::{
        basic::dentry::Dentry,
        impls::{
            proc::{
                fd::{dentry::FdDentry, inode::FdDirInode},
                interrupts::{dentry::InterruptsDentry, inode::InterruptsInode},
                maps::{dentry::MapsDentry, inode::MapsInode},
                mounts::inode::MountsInode,
                stat::{dentry::ProcStatDentry, inode::ProcStatInode},
            },
            ramfs::{
                dentry::RamFsDentry,
                inode::{RamFsDirInode, RamFsFileInode},
            },
        },
    },
    include::fs::FileFlags,
    syscall::SysResult,
};

mod exe;
mod fd;
pub mod filesystem;
mod interrupts;
mod maps;
mod meminfo;
mod mounts;
pub mod stat;
pub mod status;
mod superblock;

pub use interrupts::inc_interrupts_count;

pub async fn init(fs_root: Arc<dyn Dentry>) -> SysResult<()> {
    assert_eq!(fs_root.name(), "proc");

    info!("[fs] create /proc/meminfo");
    let mem_info_dentry = Arc::new(MemInfoDentry::new(
        Some(fs_root.clone()),
        "meminfo",
        fs_root.super_block(),
    ));
    let mem_info_inode = Arc::new(MemInfoInode::new(fs_root.super_block()));
    mem_info_dentry.into_dyn().set_inode(mem_info_inode);
    fs_root.add_child(mem_info_dentry);

    info!("[fs] create /proc/mounts");
    let mounts_dentry = Arc::new(MountsDentry::new(
        Some(fs_root.clone()),
        "mounts",
        fs_root.super_block(),
    ));
    let mounts_inode = Arc::new(MountsInode::new(fs_root.super_block()));
    mounts_dentry.into_dyn().set_inode(mounts_inode);
    fs_root.add_child(mounts_dentry);

    info!("[fs] create /proc/sys");
    let sys_dentry = Arc::new(RamFsDentry::new(
        Some(fs_root.clone()),
        "sys",
        fs_root.super_block(),
    ))
    .into_dyn();
    let sys_inode = Arc::new(RamFsDirInode::new(fs_root.super_block(), 0));
    sys_dentry.set_inode(sys_inode);
    fs_root.add_child(sys_dentry.clone());

    info!("[fs] create /proc/interrupts");
    let interrupts_dentry = Arc::new(InterruptsDentry::new(
        Some(fs_root.clone()),
        "interrupts",
        fs_root.super_block(),
    ));
    let interrupts_inode = Arc::new(InterruptsInode::new(fs_root.super_block()));
    interrupts_dentry.into_dyn().set_inode(interrupts_inode);
    fs_root.add_child(interrupts_dentry);

    info!("[fs] create /proc/sys/kernel/pid_max, write 32768");
    let kernel_inode = Arc::new(RamFsDirInode::new(sys_dentry.super_block(), 0));
    let kernel_dentry = sys_dentry.add_child_with_inode("kernel", kernel_inode);
    let pid_max_inode = Arc::new(RamFsFileInode::new(kernel_dentry.super_block(), 0));
    let pid_max_dentry = kernel_dentry.add_child_with_inode("pid_max", pid_max_inode);
    pid_max_dentry
        .open(&FileFlags::O_WRONLY)?
        .write_at(0, "32768\0".as_bytes())
        .await?;

    info!("[fs] create /proc/self");
    let self_dentry: Arc<dyn Dentry> = Arc::new(RamFsDentry::new(
        Some(fs_root.clone()),
        "self",
        fs_root.super_block(),
    ))
    .into_dyn();
    let self_inode = Arc::new(RamFsDirInode::new(fs_root.super_block(), 0));
    self_dentry.set_inode(self_inode);
    fs_root.add_child(self_dentry.clone());

    info!("[fs] create /proc/self/exe");
    let exe_dentry = Arc::new(ExeDentry::new(
        Some(fs_root.clone()),
        "exe",
        fs_root.super_block(),
    ));
    let exe_inode = Arc::new(ExeInode::new(fs_root.super_block()));
    exe_dentry.into_dyn().set_inode(exe_inode);
    self_dentry.add_child(exe_dentry);

    info!("[fs] create /proc/self/status");
    let status_dentry = Arc::new(StatusDentry::new(
        Some(self_dentry.clone()),
        "status",
        fs_root.super_block(),
    ));
    let status_inode = Arc::new(StatusInode::new(fs_root.super_block()));
    status_dentry.into_dyn().set_inode(status_inode);
    self_dentry.add_child(status_dentry);

    info!("[fs] create /proc/self/stat");
    let stat_dentry = Arc::new(ProcStatDentry::new(
        Some(self_dentry.clone()),
        "stat",
        fs_root.super_block(),
    ));
    let stat_inode = Arc::new(ProcStatInode::new(fs_root.super_block()));
    stat_dentry.into_dyn().set_inode(stat_inode);
    self_dentry.add_child(stat_dentry);

    info!("[fs] create /proc/self/fd");
    let fd_dentry = Arc::new(FdDentry::new(
        Some(self_dentry.clone()),
        "fd",
        fs_root.super_block(),
    ));
    let fd_inode = Arc::new(FdDirInode::new(fs_root.super_block()));
    fd_dentry.into_dyn().set_inode(fd_inode);
    self_dentry.add_child(fd_dentry);

    info!("[fs] create /proc/self/maps");
    let maps_dentry = Arc::new(MapsDentry::new(
        Some(self_dentry.clone()),
        "maps",
        fs_root.super_block(),
    ));
    let maps_inode = Arc::new(MapsInode::new(fs_root.super_block()));
    maps_dentry.into_dyn().set_inode(maps_inode);
    self_dentry.add_child(maps_dentry);

    Ok(())
}

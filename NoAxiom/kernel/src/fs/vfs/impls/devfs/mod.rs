use alloc::sync::Arc;

use cpu_dma_latency::{dentry::CpuDmaLatencyDentry, inode::CpuDmaLatencyInode};
use ksync::Once;
use rtc::{dentry::RtcDentry, inode::RtcInode};
use tty::{dentry::TtyDentry, inode::TtyInode};
use urandom::{dentry::UrandomDentry, inode::UrandomInode};

use crate::{
    fs::vfs::{
        basic::{dentry::Dentry, file::File},
        impls::{
            devfs::{
                loop_control::{dentry::LoopControlDentry, inode::LoopControlInode},
                null::{NullDentry, NullInode},
            },
            ramfs::{dentry::RamFsDentry, inode::RamFsDirInode},
        },
    },
    include::fs::FileFlags,
    syscall::SysResult,
};

pub mod cpu_dma_latency;
pub mod filesystem;
pub mod loop_control;
pub mod loopdev;
pub mod null;
pub mod rtc;
pub mod superblock;
pub mod tty;
pub mod urandom;
pub mod zero;

pub static TTYFILE: Once<Arc<dyn File>> = Once::new();

pub async fn init(fs_root: Arc<dyn Dentry>) -> SysResult<()> {
    assert_eq!(fs_root.name(), "dev");

    info!("[fs] create /dev/null");
    let null_dentry = Arc::new(NullDentry::new(
        Some(fs_root.clone()),
        "null",
        fs_root.super_block(),
    ));
    let null_inode = Arc::new(NullInode::new(fs_root.super_block()));
    null_dentry.into_dyn().set_inode(null_inode);
    fs_root.add_child(null_dentry);

    info!("[fs] create /dev/zero");
    let zero_dentry = Arc::new(zero::dentry::ZeroDentry::new(
        Some(fs_root.clone()),
        "zero",
        fs_root.super_block(),
    ));
    let zero_inode = Arc::new(zero::inode::ZeroInode::new(fs_root.super_block()));
    zero_dentry.into_dyn().set_inode(zero_inode);
    fs_root.add_child(zero_dentry);

    info!("[fs] create /dev/tty");
    let tty_dentry = Arc::new(TtyDentry::new(
        Some(fs_root.clone()),
        "tty",
        fs_root.super_block(),
    ));
    let tty_inode = Arc::new(TtyInode::new(fs_root.super_block()));
    tty_dentry.into_dyn().set_inode(tty_inode);
    fs_root.add_child(tty_dentry.clone());

    let tty_file = tty_dentry.open(&FileFlags::O_RDWR)?;
    TTYFILE.call_once(|| tty_file);

    info!("[fs] create /dev/rtc");
    let rtc_dentry = Arc::new(RtcDentry::new(
        Some(fs_root.clone()),
        "rtc",
        fs_root.super_block(),
    ));
    let rtc_inode = Arc::new(RtcInode::new(fs_root.super_block()));
    rtc_dentry.into_dyn().set_inode(rtc_inode);
    fs_root.add_child(rtc_dentry);

    info!("[fs] create /dev/cpu_dma_latency");
    let cpu_dma_latency_dentry = Arc::new(CpuDmaLatencyDentry::new(
        Some(fs_root.clone()),
        "cpu_dma_latency",
        fs_root.super_block(),
    ));
    let cpu_dma_latency_inode = Arc::new(CpuDmaLatencyInode::new(fs_root.super_block()));
    cpu_dma_latency_dentry
        .into_dyn()
        .set_inode(cpu_dma_latency_inode);
    fs_root.add_child(cpu_dma_latency_dentry);

    info!("[fs] create /dev/urandom");
    let urandom_dentry = Arc::new(UrandomDentry::new(
        Some(fs_root.clone()),
        "urandom",
        fs_root.super_block(),
    ));
    let urandom_inode = Arc::new(UrandomInode::new(fs_root.super_block()));
    urandom_dentry.into_dyn().set_inode(urandom_inode);
    fs_root.add_child(urandom_dentry);

    //todo: add /dev/shm
    info!("[fs] create /dev/shm");
    let shm_dentry = Arc::new(RamFsDentry::new(
        Some(fs_root.clone()),
        "shm",
        fs_root.super_block(),
    ));
    let shm_inode = Arc::new(RamFsDirInode::new(fs_root.super_block(), 0));
    shm_dentry.into_dyn().set_inode(shm_inode);
    fs_root.add_child(shm_dentry.clone());

    //todo: add /dev/misc

    info!("[fs] create /dev/loop-control");
    let loop_control_dentry = Arc::new(LoopControlDentry::new(
        Some(fs_root.clone()),
        "loop-control",
        fs_root.super_block(),
    ));
    let loop_control_inode = Arc::new(LoopControlInode::new(fs_root.super_block()));
    loop_control_dentry.into_dyn().set_inode(loop_control_inode);
    fs_root.add_child(loop_control_dentry);

    Ok(())
}

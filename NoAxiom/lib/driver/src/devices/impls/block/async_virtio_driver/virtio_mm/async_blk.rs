//! virtio异步块设备驱动

use alloc::boxed::Box;

use arch::ArchMemory;
use async_trait::async_trait;
use ksync::mutex::check_no_lock;
#[cfg(feature = "qemu")]
use platform::qemu::VIRTIO0 as VIRTIO0_VIRT;

use crate::devices::impls::{
    block::{
        async_virtio_driver::{
            block::{InterruptRet, VirtIOBlock},
            mmio::VirtIOHeader,
        },
        BlockDevice,
    },
    device::{DevResult, Device},
};

const VIRTIO0: usize = VIRTIO0_VIRT | arch::Arch::KERNEL_ADDR_OFFSET;

/// 异步虚拟块设备接口
///
/// note: [`VirtIOBlock`]中的常量泛型参数指代一个块中有多少个扇区，
/// 这里目前假定为一个块对应一个扇区
pub struct VirtIOAsyncBlock(pub VirtIOBlock<1>);

impl VirtIOAsyncBlock {
    #[allow(unused)]
    pub async fn async_new() -> VirtIOAsyncBlock {
        let header = unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) };
        let async_blk = VirtIOBlock::async_new(header).await.unwrap();
        Self(async_blk)
    }
    /// 创建一个[`VirtIOAsyncBlock`]
    pub fn new() -> Self {
        let header = unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) };
        let blk = VirtIOBlock::new(header).unwrap();
        Self(blk)
    }
    /// 从virtio块设备中读取一个块
    ///
    /// 该async函数在执行器中第一次被`poll`的时候返回`Pending`，
    /// virtio外部中断来了会在中断处理函数里面唤醒
    ///
    /// # Example:
    ///
    /// ```
    /// # const BLOCK_SIZE: usize = 512;
    /// async {
    ///     let mut buf = [0u8; BLOCK_SIZE];
    ///     // 读第一个块中的数据
    ///     VIRTIO_BLOCK.read_block(0, &mut buf).await;    
    /// }
    /// ```
    #[allow(unused)]
    pub async fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .read_block_event(block_id, buf)
            .await
            .expect("read block with event");
    }
    /// 往virtio块设备中写入一个块
    ///
    /// 该async函数在执行器中第一次被`poll`的时候返回`Pending`，
    /// virtio外部中断来了会在中断处理函数里面唤醒
    ///
    /// # Example:
    ///
    /// ```
    /// # const BLOCK_SIZE: usize = 512;
    /// async {
    ///     let buf = [1u8; BLOCK_SIZE];
    ///     // 读第一个块中的数据
    ///     VIRTIO_BLOCK.write_block(0, &buf).await;    
    /// }
    /// ```
    pub async fn write_block(&self, block_id: usize, buf: &[u8]) {
        // self.0.async_write_block(block_id, buf).await.expect("failed to write block
        // from async_virtio_block!");
        self.0
            .write_block_event(block_id, buf)
            .await
            .expect("write block with event");
    }
    /// 处理virtio外部中断，通常在外部中断处理函数里面使用
    ///
    /// # Example:
    ///
    /// ```
    /// unsafe extern "C" fn supervisor_external() {
    ///     let irq = plic::plic_claim();
    ///     if irq == 1 {
    ///         let ret = VIRTIO_BLOCK.handle_interrupt().unwrap();
    ///         println!("virtio intr return: {}", ret);        
    ///     }
    /// }
    /// ```
    pub unsafe fn handle_interrupt(&self) -> Option<u64> {
        let ret = self
            .0
            .handle_interrupt()
            .expect("handle virtio interrupt error!");
        match ret {
            InterruptRet::Read(sector) => {
                return Some(sector);
            }
            InterruptRet::Write(sector) => {
                return Some(sector);
            }
            _other => {
                return None;
            }
        }
    }
}

#[async_trait]
impl Device for VirtIOAsyncBlock {
    fn device_name(&self) -> &'static str {
        "virtio_async_block"
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        assert!(check_no_lock());
        self.read_block(id, buf).await;
        Ok(buf.len())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        assert!(check_no_lock());
        self.write_block(id, buf).await;
        Ok(buf.len())
    }
}

impl BlockDevice for VirtIOAsyncBlock {}

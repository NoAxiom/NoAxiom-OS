pub mod sata;
pub mod virtio;
use alloc::sync::Arc;

use ksync::Once;

use crate::device::block::virtio::virtio as Virtio;

#[cfg(target_arch = "riscv64")]
type BlockDeviceImpl = Virtio;
#[cfg(target_arch = "loongarch64")]
type BlockDeviceImpl = Virtio;

pub static BLOCK_DEVICE: Once<Arc<BlockDeviceImpl>> = Once::new();

pub fn init_block_device(block_device: Arc<BlockDeviceImpl>) {
    BLOCK_DEVICE.call_once(|| block_device);
}
use alloc::boxed::Box;

use async_trait::async_trait;

#[async_trait]
pub trait BlockDevice: Send + Sync {
    async fn read(&self, id: usize, buf: &mut [u8]);
    async fn write(&self, id: usize, buf: &[u8]);
    async fn sync_all(&self);
}

pub mod virtio_gpu;
use alloc::boxed::Box;

use crate::devices::basic::Device;

#[async_trait::async_trait]
pub trait DisplayDevice: Send + Sync + Device {
    fn update_cursor(&self);
    fn get_framebuffer(&self) -> &mut [u8];
    async fn flush(&self);
}

pub enum DisplayDriverType {
    Virtio,
    PhysRV,
    PhysLA,
}

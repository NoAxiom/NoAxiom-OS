pub mod virtio_gpu;
use alloc::boxed::Box;

#[async_trait::async_trait]
pub trait DisplayDevice: Send + Sync {
    fn update_cursor(&self);
    fn get_framebuffer(&self) -> &mut [u8];
    async fn flush(&self);
}

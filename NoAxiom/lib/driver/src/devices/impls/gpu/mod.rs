pub mod virtio_gpu;

pub trait Gpu {
    fn update_cursor(&self);
    fn get_framebuffer(&self) -> &mut [u8];
    fn flush(&self);
}

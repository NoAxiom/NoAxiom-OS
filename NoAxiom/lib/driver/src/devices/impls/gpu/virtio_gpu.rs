use core::ptr::NonNull;

use config::mm::VIRTIO7;
use ksync::mutex::SpinLock;
use virtio_drivers::{
    device::gpu::VirtIOGpu,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

use super::Gpu;
use crate::devices::impls::virtio::VirtioHalImpl;

/// Virtio GPU device at MMIO bus
pub struct VirtioGpu {
    gpu: SpinLock<VirtIOGpu<VirtioHalImpl, MmioTransport>>,
    fb: &'static [u8],
}

impl VirtioGpu {
    pub fn new() -> Self {
        unsafe {
            let header = NonNull::new(VIRTIO7 as *mut VirtIOHeader).unwrap();
            let transport = MmioTransport::new(header).unwrap();
            let mut virtio = VirtIOGpu::new(transport).unwrap();
            let fbuffer = virtio.setup_framebuffer().unwrap();
            let len = fbuffer.len();
            let ptr = fbuffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);
            Self {
                gpu: SpinLock::new(virtio),
                fb,
            }
        }
    }
}

impl Gpu for VirtioGpu {
    fn flush(&self) {
        self.gpu.lock().flush().unwrap();
    }
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }
    fn update_cursor(&self) {}
}

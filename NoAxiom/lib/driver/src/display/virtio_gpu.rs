use alloc::boxed::Box;
use core::ptr::NonNull;

use arch::consts::IO_ADDR_OFFSET;
use ksync::mutex::SpinLock;
use virtio_drivers::{
    device::gpu::VirtIOGpu,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

use super::DisplayDevice;
use crate::{basic::Device, hal::VirtioHalImpl};

/// Virtio GPU device at MMIO bus
pub struct VirtioGpu {
    gpu: SpinLock<VirtIOGpu<VirtioHalImpl, MmioTransport>>,
    fb: &'static [u8],
}

impl VirtioGpu {
    pub async fn new(addr: usize, size: usize) -> Self {
        unsafe {
            let virtio7 = addr | IO_ADDR_OFFSET;
            let header = NonNull::new(virtio7 as *mut VirtIOHeader).unwrap();
            // fixme: | kernel addr offset
            let transport = MmioTransport::new(header, size).unwrap();
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

impl Device for VirtioGpu {
    fn device_name(&self) -> &'static str {
        "Virtio GPU"
    }
    fn device_type(&self) -> &'static crate::basic::DeviceType {
        &crate::basic::DeviceType::Display(
            crate::basic::DisplayDeviceType::Virtio,
        )
    }
}

#[async_trait::async_trait]
impl DisplayDevice for VirtioGpu {
    async fn flush(&self) {
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

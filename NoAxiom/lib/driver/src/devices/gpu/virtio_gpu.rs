use alloc::boxed::Box;
use core::ptr::NonNull;

use arch::{Arch, ArchMemory};
use ksync::mutex::SpinLock;
use virtio_drivers::{
    device::gpu::VirtIOGpu,
    transport::mmio::{MmioTransport, VirtIOHeader},
};

use super::DisplayDevice;
use crate::{devices::hal::VirtioHalImpl, dtb::basic::dtb_info};

/// Virtio GPU device at MMIO bus
pub struct VirtioGpu {
    gpu: SpinLock<VirtIOGpu<VirtioHalImpl, MmioTransport>>,
    fb: &'static [u8],
}

impl VirtioGpu {
    pub async fn new() -> Self {
        unsafe {
            let (virtio7_paddr, size) = dtb_info().virtio_mmio_regions[7];
            let virtio7 = virtio7_paddr | Arch::KERNEL_ADDR_OFFSET;
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

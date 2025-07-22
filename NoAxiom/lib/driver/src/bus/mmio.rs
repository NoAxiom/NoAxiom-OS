use core::ptr::NonNull;

use ksync::Lazy;
use platform::dtb::basic::dtb_info;
use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};

use crate::devices::block::virtio_block::VirtioBlockDevice;

static MMIO_BLOCK_DEVICE: Lazy<Option<VirtioBlockDevice<MmioTransport>>> = Lazy::new(|| {
    let dtb_info = dtb_info();
    if dtb_info.virtio_mmio_regions.is_empty() {
        return None;
    }

    let (addr, size) = dtb_info.virtio_mmio_regions[0];
    log::info!("[driver] probe virtio wrapper at {:#x}", addr);
    let addr = addr | arch::consts::KERNEL_ADDR_OFFSET;
    let header = NonNull::new(addr as *mut VirtIOHeader).unwrap();
    let transport = unsafe { MmioTransport::new(header, size).unwrap() };

    Some(VirtioBlockDevice::new(transport))
});

pub fn probe_mmiobus_devices() -> Option<&'static VirtioBlockDevice<MmioTransport>> {
    MMIO_BLOCK_DEVICE.as_ref()
}

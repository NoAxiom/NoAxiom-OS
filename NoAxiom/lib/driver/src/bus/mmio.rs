use crate::{
    devices::{impls::device::DevResult, Devices},
    dtb::dtb_info,
};

impl Devices {
    pub fn probe_mmiobus_devices(&mut self) -> DevResult<()> {
        let mut registered: [bool; Self::DEVICES] = [false; Self::DEVICES];
        for (addr, _size) in &dtb_info().virtio_mmio_regions {
            if !registered[0] {
                #[cfg(feature = "async")]
                {
                    log::debug!("[driver] probe driver");
                    use core::ptr::NonNull;

                    use include::errno::Errno;
                    use spin::Mutex;
                    use virtio_drivers_async::{
                        device::blk::VirtIOBlk,
                        transport::mmio::{MmioTransport, VirtIOHeader},
                    };

                    use crate::devices::impls::{virtio::dev_err, BlkDevice};

                    let transport = unsafe {
                        MmioTransport::new(NonNull::new(*addr as *mut VirtIOHeader).unwrap())
                            .map_err(|_| Errno::EINVAL)?
                    };

                    let blk_dev =
                        BlkDevice::Mmio(Mutex::new(VirtIOBlk::new(transport).map_err(dev_err)?));

                    self.add_blk_device(blk_dev);
                }
                #[cfg(feature = "interruptable_async")]
                {
                    log::debug!("[driver] probe async driver");
                    use crate::devices::impls::block::async_virtio_driver::virtio_mm::async_blk::VirtIOAsyncBlock;
                    let _ = addr;
                    let blk_dev = VirtIOAsyncBlock::new();
                    self.add_blk_device(blk_dev);
                }
                registered[0] = true;
            }
        }
        Ok(())
    }
}

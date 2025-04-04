use crate::{
    devices::{impls::device::DevResult, Devices},
    dtb::dtb_info,
};

impl Devices {
    pub fn probe_mmiobus_devices(&mut self) -> DevResult<()> {
        let mut registered: [bool; Self::DEVICES] = [false; Self::DEVICES];
        for (addr, _size) in &dtb_info().mmio_regions {
            if !registered[0] {
                #[cfg(not(all(feature = "async_fs", target_arch = "riscv64")))]
                {
                    use core::ptr::NonNull;

                    use include::errno::Errno;
                    use virtio_drivers::transport::mmio::{MmioTransport, VirtIOHeader};

                    use crate::devices::impls::{block::virtio_block::VirtioBlock, BlkDevice};

                    let blk_dev = BlkDevice::Mmio(VirtioBlock::<MmioTransport>::try_new(unsafe {
                        MmioTransport::new(NonNull::new(*addr as *mut VirtIOHeader).unwrap())
                            .map_err(|_| Errno::EINVAL)?
                    })?);

                    self.add_blk_device(blk_dev);
                }
                #[cfg(all(feature = "async_fs", target_arch = "riscv64"))]
                {
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

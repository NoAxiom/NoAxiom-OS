use alloc::{sync::Arc, vec::Vec};
use core::ptr::NonNull;

use arch::{consts::KERNEL_ADDR_OFFSET, Arch, DtbInfo};
use fdt::Fdt;
use ksync::mutex::SpinLock;
use virtio_drivers::transport::{
    mmio::{MmioTransport, VirtIOHeader},
    pci::PciTransport,
    DeviceType, Transport,
};

use crate::{
    device::block::{sata::SataBlock, virtio::PciVirtio, BlockDevice},
    driver::{
        block::{
            virtio::{virtio_blk::VirtIOBlockDriver, virtio_impl::HalImpl},
            BlockDriver,
        },
        probe::{Probe, ProbeInfo, PROBE},
        Driver, DRIVER_MANAGER,
    },
    platform,
};

pub fn device_init() {
    Probe::init();

    #[cfg(not(all(feature = "vf2", feature = "hifive")))]
    {
        match PROBE.get().unwrap().probe_virtio() {
            Some(virtio_mmio_devices) => {
                init_virtio_mmio(virtio_mmio_devices);
            }
            None => {
                // super::block::init_block_device(Arc::new(SataBlock::new()));
                #[cfg(target_arch = "loongarch64")]
                {
                    let res = crate::device::pci::init();
                    if let Ok(blk) = res {
                        let pci_blk = PciVirtio::new(blk);
                        super::block::init_block_device(Arc::new(pci_blk));
                        debug!("Init PCI block device success");
                        return;
                    }
                    panic!("There is no block device");
                }
            }
        }
    }

    #[cfg(feature = "vf2")]
    match PROBE.get().unwrap().probe_sdio() {
        Some(sdio) => {
            println!("vf2 into probe");
            init_block_device(sdio, None)
        }
        None => {
            panic!("There is no sdio device");
        }
    }
    #[cfg(feature = "uart")]
    match PROBE.get().unwrap().probe_uart() {
        Some(uart) => {
            println!("vf2 into probe");

            init_uart(uart);
            interrupt::register_device_to_plic(
                uart.property("interrupts").unwrap().as_usize().unwrap(),
                UART_DEVICE.get().unwrap().clone() as Arc<dyn Device>,
            )
        }
        None => {
            panic!("There is no sdio device");
        }
    }
    // #[cfg(any(feature = "hifive", feature = "vf2"))]
    // init_net(None);
}

pub fn init_virtio_mmio(devices: Vec<ProbeInfo>) {
    for device in devices {
        // println!("name : {}", device.name);
        let paddr = device.base_addr + KERNEL_ADDR_OFFSET;
        // println!("device.base_addr:{:x}", paddr);
        if paddr != ::platform::qemu::VIRTIO0 {
            // println!("paddr : {:x}", paddr);
            // todo: can be optimized
            continue;
        }

        let header = NonNull::new(paddr as *mut VirtIOHeader).unwrap();
        // println!("header:{:?}", header);

        match unsafe { MmioTransport::new(header) } {
            Err(_) => {
                println!("header err");
            }
            Ok(mut transport) => {
                info!(
                    "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}, features:{:?}",
                    transport.vendor_id(),
                    transport.device_type(),
                    transport.version(),
                    transport.read_device_features(),
                );
                match transport.device_type() {
                    // DeviceType::Input => {
                    //     if paddr == VIRTIO5 {
                    //         init_input_device(device, "keyboard", Some(transport));
                    //     } else if paddr == VIRTIO6 {
                    //         init_input_device(device, "mouse", Some(transport));
                    //     }
                    // }
                    DeviceType::Block => {
                        init_block_device(device, Some(transport));
                    }

                    // DeviceType::GPU => init_gpu(device, Some(transport)),
                    // DeviceType::Network => init_net(Some(device)),
                    ty => {
                        println!("Don't support virtio device type: {:?}", ty);
                    }
                }
            }
        }
    }
}

fn init_block_device(blk: ProbeInfo, mmio_transport: Option<MmioTransport>) {
    let (base_addr, irq) = (blk.base_addr, blk.irq);
    match blk.compatible.as_str() {
        "virtio,mmio" => {
            #[cfg(feature = "async_fs")]
            {
                debug!("Block initialization will be performed on the first read event");
            }
            #[cfg(not(feature = "async_fs"))]
            {
                let block_driver = VirtIOBlockDriver::from_mmio(mmio_transport.unwrap());
                let size = block_driver.size();
                info!(
                    "Init block device which size is {}MB, base_addr:{:#x}, irq:{}",
                    size * 512 / 1024 / 1024,
                    base_addr,
                    irq
                );
                let driver: Arc<dyn Driver> = Arc::new(block_driver);
                DRIVER_MANAGER.lock().push_driver(driver.clone());
                let driver = Some(Arc::downgrade(&driver));

                let block_device =
                    Arc::new(super::block::virtio::virtio::new(driver, base_addr, size));

                super::block::init_block_device(block_device);
                // register_device_to_plic(irq, block_device);
                debug!("Init block device success");
            }
        }
        "starfive,jh7110-sdio" => {
            // starfive2
            #[cfg(not(feature = "ramdisk"))]
            {
                // let sd = SdIoImpl;
                // let sleep = SleepOpsImpl;

                // todo: implement sdio driver
                // let sd = Vf2SdDriver::<_, SleepOpsImpl>::new(SdIoImpl);
                // let driver = VF2SDDriver::new(sd);
                // println!("Init block device, base_addr:{:#x},irq:{}",
                // base_addr, irq); let size = driver.size();
                // println!("Block device size is {}MB", size);
                // let driver: Arc<dyn Driver> = Arc::new(driver);
                // DRIVER_MANAGER.lock().push_driver(driver.clone());
                // let driver = Some(Arc::downgrade(&driver));
                // let block_device =
                //     Arc::new(super::Block::vf2sd::vfs2d::new(driver,
                // base_addr, size));
                // super::Block::init_block_device(block_device);
                // // register_device_to_plic(irq, block_device);
                // println!("Init SDIO block device success");
            }
            #[cfg(feature = "ramdisk")]
            {
                init_ramdisk();
            }
        }
        name => {
            println!("Don't support block device: {}", name);
            #[cfg(feature = "ramdisk")]
            {
                init_ramdisk();
            }
            #[cfg(not(feature = "ramdisk"))]
            panic!("System need block device, but there is no block device");
        }
    }
}

// pub trait Device: Send + Sync {
//     fn name(&self) -> &str;
//     fn dev_type(&self) -> DeviceType;
//     /// Register base address
//     fn mmio_base(&self) -> usize;
//     fn mmio_size(&self) -> usize;
//     fn interrupt_number(&self) -> Option<usize>;
//     fn interrupt_handler(&self);
//     fn init(&self);
// fn driver(&self) -> Option<Arc<dyn Driver>>;
// fn set_driver(&self, driver: Option<Weak<dyn Driver>>);
// fn is_dead(&self) -> bool;
// fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlockDevice>>;
// fn as_char(self: Arc<Self>) -> Option<Arc<dyn CharDevice>>;
// }

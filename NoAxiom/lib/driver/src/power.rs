pub trait PowerDevice: Device {
    fn shutdown() -> !;
}

#[cfg(target_arch = "loongarch64")]
mod la_virtio {
    use arch::{consts::IO_ADDR_OFFSET, Arch, ArchAsm, ArchInt};

    use crate::{basic::Device, power::PowerDevice};

    const GED_PADDR: usize = 0x100E_001C;
    fn get_ged_addr() -> usize {
        GED_PADDR | IO_ADDR_OFFSET
    }
    fn shutdown() -> ! {
        let ptr = get_ged_addr() as *mut u8;
        // Shutdown the whole system, including all CPUs.
        unsafe { ptr.write_volatile(0x34) };
        loop {
            Arch::disable_interrupt();
            Arch::set_idle();
        }
    }

    pub struct PowerDev;
    impl PowerDevice for PowerDev {
        fn shutdown() -> ! {
            shutdown()
        }
    }
    impl Device for PowerDev {
        fn device_name(&self) -> &'static str {
            "Power"
        }
        fn device_type(&self) -> &'static crate::basic::DeviceType {
            &crate::basic::DeviceType::Power(crate::basic::PowerDeviceType::Virtio)
        }
    }
}

#[cfg(target_arch = "loongarch64")]
pub use la_virtio::*;

#[cfg(target_arch = "riscv64")]
mod rv {
    use crate::{basic::Device, power::PowerDevice};

    pub struct PowerDev;
    impl PowerDevice for PowerDev {
        fn shutdown() -> ! {
            sbi_rt::legacy::shutdown()
        }
    }
    impl Device for PowerDev {
        fn device_name(&self) -> &'static str {
            "Power"
        }
        fn device_type(&self) -> &'static crate::basic::DeviceType {
            &crate::basic::DeviceType::Power(crate::basic::PowerDeviceType::Virtio)
        }
    }
}

#[cfg(target_arch = "riscv64")]
pub use rv::*;

use crate::basic::Device;

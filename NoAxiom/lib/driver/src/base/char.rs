pub trait CharDevice: Device {
    fn putchar(c: u8);
    fn getchar() -> u8;
}

#[cfg(target_arch = "loongarch64")]
mod la_virtio {
    use arch::consts::IO_ADDR_OFFSET;

    use crate::{base::char::CharDevice, basic::Device};

    /// No initialization required Devices, but also from dtb info
    #[cfg(feature = "qemu")]
    const UART_PADDR: usize = 0x1FE0_01E0; // qemu-virt
    #[cfg(not(feature = "qemu"))]
    const UART_PADDR: usize = 0x1FE2_0000; // qemu-2k1000

    #[inline(always)]
    const fn get_com1_addr() -> usize {
        UART_PADDR | IO_ADDR_OFFSET
    }

    pub struct CharDev;
    impl CharDev {
        pub fn base_putchar(c: u8) {
            let ptr = get_com1_addr() as *mut u8;
            loop {
                unsafe {
                    if ptr.add(5).read_volatile() & (1 << 5) != 0 {
                        break;
                    }
                }
            }
            unsafe {
                ptr.add(0).write_volatile(c);
            }
        }
        pub fn base_getchar() -> Option<u8> {
            let ptr = get_com1_addr() as *mut u8;
            unsafe {
                if ptr.add(5).read_volatile() & 1 == 0 {
                    // The DR bit is 0, meaning no data
                    None
                } else {
                    // The DR bit is 1, meaning data!
                    Some(ptr.add(0).read_volatile())
                }
            }
        }
    }

    impl CharDevice for CharDev {
        #[inline]
        fn putchar(c: u8) {
            if c == b'\n' {
                Self::base_putchar(b'\r');
            }
            Self::base_putchar(c)
        }

        /// read a byte, return -1 if nothing exists.
        #[inline]
        fn getchar() -> u8 {
            loop {
                if let Some(c) = Self::base_getchar() {
                    return c;
                }
            }
        }
    }
    impl Device for CharDev {
        fn device_name(&self) -> &'static str {
            "char"
        }
        fn device_type(&self) -> &'static crate::basic::DeviceType {
            &crate::basic::DeviceType::Char(crate::basic::CharDeviceType::Virtio)
        }
    }
}

#[cfg(target_arch = "loongarch64")]
pub use la_virtio::*;

#[cfg(target_arch = "riscv64")]
mod rv {
    use crate::{base::char::CharDevice, basic::Device};

    pub struct CharDev;
    impl CharDevice for CharDev {
        #[inline]
        fn putchar(ch: u8) {
            sbi_rt::legacy::console_putchar(ch as usize);
        }

        /// read a byte, return -1 if nothing exists.
        #[inline]
        fn getchar() -> u8 {
            sbi_rt::legacy::console_getchar() as u8
        }
    }

    impl Device for CharDev {
        fn device_name(&self) -> &'static str {
            "char"
        }
        fn device_type(&self) -> &'static crate::basic::DeviceType {
            &crate::basic::DeviceType::Char(crate::basic::CharDeviceType::Virtio)
        }
    }
}

#[cfg(target_arch = "riscv64")]
pub use rv::*;

use crate::basic::Device;

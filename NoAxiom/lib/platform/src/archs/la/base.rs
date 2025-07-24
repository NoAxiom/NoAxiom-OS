use arch::{consts::IO_ADDR_OFFSET, Arch, ArchAsm, ArchInt};

use crate::archs::common::base::BaseFu;

/// No initialization required Devices, but also from dtb info
const GED_PADDR: usize = 0x100E_001C;
const UART_PADDR: usize = 0x1FE0_01E0;

pub fn get_com1_addr() -> usize {
    UART_PADDR | IO_ADDR_OFFSET
}

pub fn get_ged_addr() -> usize {
    GED_PADDR | IO_ADDR_OFFSET
}

pub struct Base;
impl Base {
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

impl BaseFu for Base {
    #[inline]
    fn putchar(c: u8) {
        if c == b'\n' {
            Base::base_putchar(b'\r');
        }
        Base::base_putchar(c)
    }

    /// read a byte, return -1 if nothing exists.
    #[inline]
    fn getchar() -> u8 {
        loop {
            if let Some(c) = Base::base_getchar() {
                return c;
            }
        }
    }

    #[inline]
    fn shutdown() -> ! {
        let ptr = get_ged_addr() as *mut u8;
        // Shutdown the whole system, including all CPUs.
        unsafe { ptr.write_volatile(0x34) };
        loop {
            Arch::disable_interrupt();
            Arch::set_idle();
        }
    }
}

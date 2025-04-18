use arch::{consts::KERNEL_ADDR_OFFSET, Arch, ArchAsm, ArchInt};

use super::BaseFu;
use crate::{GED_PADDR, UART_PADDR};
const COM1_ADDR: usize = UART_PADDR | KERNEL_ADDR_OFFSET;
const GED_ADDR: usize = GED_PADDR | KERNEL_ADDR_OFFSET;

pub struct Base;
impl Base {
    pub fn base_putchar(c: u8) {
        let ptr = COM1_ADDR as *mut u8;
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
        let ptr = COM1_ADDR as *mut u8;
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
        let ptr = GED_ADDR as *mut u8;
        // Shutdown the whole system, including all CPUs.
        unsafe { ptr.write_volatile(0x34) };
        loop {
            Arch::disable_interrupt();
            Arch::set_idle();
        }
    }
}

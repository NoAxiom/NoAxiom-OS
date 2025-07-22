use crate::archs::common::base::BaseFu;

pub struct Base;

impl BaseFu for Base {
    #[inline]
    fn putchar(ch: u8) {
        sbi_rt::legacy::console_putchar(ch as usize);
    }

    /// read a byte, return -1 if nothing exists.
    #[inline]
    fn getchar() -> u8 {
        sbi_rt::legacy::console_getchar() as u8
    }

    #[inline]
    fn shutdown() -> ! {
        sbi_rt::legacy::shutdown()
    }
}

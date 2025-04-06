use super::BaseFu;

pub struct Base;

impl BaseFu for Base {
    #[inline]
    fn putchar(ch: usize) {
        sbi_rt::legacy::console_putchar(ch);
    }

    /// read a byte, return -1 if nothing exists.
    #[inline]
    fn getchar() -> usize {
        sbi_rt::legacy::console_getchar()
    }

    #[inline]
    fn shutdown() -> ! {
        sbi_rt::legacy::shutdown()
    }
}

pub mod platform_info;
use core::any::Any;

use platform_info::PlatformInfo as other_platforminfo;

use crate::driver::sbi::{console_getchar, console_putchar, set_timer, shutdown};
pub trait BaseRiscv: Send + Sync + Any {
    fn init_dtb(&self, dtb: Option<usize>);
    fn base_info(&self) -> other_platforminfo;
    fn set_timer(&self, time: usize) {
        set_timer(time.try_into().unwrap());
    }
    fn system_shutdown(&self) -> ! {
        shutdown();
    }
    fn console_putchar(&self, ch: u8) {
        console_putchar(ch.into());
    }
    fn console_getchar(&self) -> isize {
        console_getchar()
    }
}

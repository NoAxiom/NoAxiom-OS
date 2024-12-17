pub mod platforminfo;
use core::any::Any;

use platforminfo::PlatformInfo as other_platforminfo;
pub trait BaseRISCV: Send + Sync + Any {
    #[allow(unused)]
    fn init_dtb(&self, dtb: Option<usize>);

    fn base_info(&self) -> other_platforminfo;

    // fn set_timer(&self, time: usize) {
    //     sbi::set_timer(time.try_into().unwrap());
    // }

    // fn system_shutdown(&self) -> ! {
    //     sbi::shutdown();
    // }

    // fn console_putchar(&self, ch: u8) {
    //     sbi::console_putchar(ch.into());
    // }

    // fn console_getchar(&self) -> isize {
    //     sbi::console_getchar()
    // }
}

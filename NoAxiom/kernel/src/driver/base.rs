use platform::archs::{base::Base, common::base::BaseFu};

pub fn getchar() -> u8 {
    Base::getchar()
}

pub fn putchar(c: u8) {
    Base::putchar(c);
}

pub fn shutdown() -> ! {
    Base::shutdown()
}

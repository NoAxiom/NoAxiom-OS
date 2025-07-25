use driver::base::{
    char::{CharDev, CharDevice},
    power::{PowerDev, PowerDevice},
};

pub fn getchar() -> u8 {
    CharDev::getchar()
}

pub fn putchar(c: u8) {
    CharDev::putchar(c);
}

pub fn shutdown() -> ! {
    PowerDev::shutdown()
}

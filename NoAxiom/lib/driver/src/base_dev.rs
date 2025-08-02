use crate::{
    char::CharDevice,
    debug::DebugCharDev,
    power::{PowerDev, PowerDevice},
};

pub fn getchar() -> u8 {
    DebugCharDev::getchar()
}

pub fn putchar(c: u8) {
    DebugCharDev::putchar(c);
}

pub fn shutdown() -> ! {
    PowerDev::shutdown()
}

pub fn debug_shutdown() -> ! {
    crate::println!("[kernel] press any key to shutdown");
    while getchar() as i8 == -1 {}
    shutdown()
}

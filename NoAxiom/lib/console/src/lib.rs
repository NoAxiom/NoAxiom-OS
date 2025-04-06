#![no_std]
#![allow(deprecated)]

use core::fmt::{self, Write};

use ksync::mutex::SpinLock;

static PRINT_MUTEX: SpinLock<Stdout> = SpinLock::new(Stdout::new());
struct Stdout;
impl Stdout {
    pub const fn new() -> Self {
        Self
    }
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            platform::putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    PRINT_MUTEX.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

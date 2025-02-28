#![no_std]
#![allow(deprecated)]

use core::fmt::{self, Write};

use arch::{Arch, ArchSbi};
use ksync::mutex::SpinLock;

static PRINT_MUTEX: SpinLock<()> = SpinLock::new(());
struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            Arch::console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    let _lock = PRINT_MUTEX.lock();
    Stdout.write_fmt(args).unwrap();
    drop(_lock);
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

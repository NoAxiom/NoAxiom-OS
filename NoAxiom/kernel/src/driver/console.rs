use core::fmt::{self, Write};

use crate::{driver::sbi::console_putchar, sync::mutex::SpinMutex};

static PRINT_MUTEX: SpinMutex<()> = SpinMutex::new(());
struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as i32);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments<'_>) {
    let _lock = PRINT_MUTEX.lock();
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::driver::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::driver::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

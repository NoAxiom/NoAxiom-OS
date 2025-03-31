use core::fmt::{self, Write};

use spin::mutex::SpinMutex;

use crate::ArchSbi;

struct Stdout;
static STDOUT: SpinMutex<Stdout> = SpinMutex::new(Stdout);

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            crate::Arch::console_putchar(c as usize);
        }
        Ok(())
    }
}

pub(crate) fn print(args: fmt::Arguments<'_>) {
    STDOUT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::utils::debug_console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::utils::debug_console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

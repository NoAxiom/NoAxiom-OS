use core::fmt::{self, Write};

use ksync::mutex::SpinLock;

use super::debug_serial::DebugCharDev;
use crate::char::CharDevice;

static DEBUG_PRINT_MUTEX: SpinLock<DebugConsole> = SpinLock::new(DebugConsole::new());

struct DebugConsole;
impl DebugConsole {
    pub const fn new() -> Self {
        Self
    }
}

impl Write for DebugConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            DebugCharDev::putchar(c as u8);
        }
        Ok(())
    }
}

pub fn debug_print(args: fmt::Arguments<'_>) {
    DEBUG_PRINT_MUTEX.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::debug::debug_print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::debug::debug_print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println_debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        #[cfg(feature = "debug_sig")]
        {
            $crate::debug::debug_print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
        }
    }
}

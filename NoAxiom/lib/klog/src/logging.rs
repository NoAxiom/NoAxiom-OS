use core::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};

use crate_interface::call_interface;
use log::{self, Level, Log, Metadata, Record};

use super::console::print;
pub static mut LOG_INITIALIZED: AtomicBool = AtomicBool::new(false);

struct KernelLogger;

#[crate_interface::def_interface]
pub trait LogInfo: Send + Sync {
    fn iodisplay(record: &Record);
    fn kernel_log(record: &Record);
}

impl Log for KernelLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() == Level::Trace || metadata.level() == Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // call_interface!(LogInfo::kernel_log(record));
            call_interface!(LogInfo::iodisplay(record));
        }
    }

    fn flush(&self) {}
}

/// Add escape sequence to print with color in Linux console
macro_rules! with_color {
    ($args:ident, $color_code:ident) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[0m", $color_code as u8, $args)
    }};
}

/// Print msg with color
pub fn print_color(args: fmt::Arguments, color_code: u8) {
    print(with_color!(args, color_code));
}

pub fn early_init_logging() {
    log::set_logger(&KernelLogger).unwrap();
    log::set_max_level(log::LevelFilter::Off);
    unsafe { LOG_INITIALIZED.store(true, Ordering::SeqCst) };
}

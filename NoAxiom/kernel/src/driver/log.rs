// ! log

use core::sync::atomic::{AtomicBool, Ordering};

use log::{self, Level, LevelFilter, Log, Metadata, Record};

use crate::cpu::{current_cpu, get_hartid};

pub static mut LOG_BOOTED: AtomicBool = AtomicBool::new(false);

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !unsafe { LOG_BOOTED.load(Ordering::SeqCst) } {
            return;
        }
        if !self.enabled(record.metadata()) {
            return;
        }
        let color = match record.level() {
            Level::Error => 31, // Red
            Level::Warn => 93,  // BrightYellow
            Level::Info => 34,  // Blue
            Level::Debug => 32, // Green
            Level::Trace => 90, // BrightBlack
        };
        println!(
            "\u{1B}[{}m[{:>5}, HART{}, TID{}] {}\u{1B}[0m",
            color,
            record.level(),
            get_hartid(),
            current_cpu()
                .task
                .as_ref()
                .map_or_else(|| 0, |task| task.tid()),
            record.args(),
        );
    }
    fn flush(&self) {}
}

pub fn log_init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("ERROR") => LevelFilter::Error,
        Some("WARN") => LevelFilter::Warn,
        Some("INFO") => LevelFilter::Info,
        Some("DEBUG") => LevelFilter::Debug,
        Some("TRACE") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
    unsafe { LOG_BOOTED.store(true, Ordering::SeqCst) };
    info!("[init] log init success");
}

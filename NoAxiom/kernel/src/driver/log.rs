// ! log

use core::sync::atomic::{fence, AtomicBool, Ordering};

use log::{self, Level, LevelFilter, Log, Metadata, Record};

use crate::{
    cpu::{current_cpu, get_hartid},
    time::gettime::get_time_ms,
};

static mut LOG_BOOTED: bool = false;
static mut LOG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_log_booted() {
    unsafe { LOG_BOOTED = true };
    fence(Ordering::SeqCst);
}
pub fn switch_log_on() {
    unsafe { LOG_ENABLED.store(true, Ordering::SeqCst) }
}
pub fn switch_log_off() {
    unsafe { LOG_ENABLED.store(false, Ordering::SeqCst) }
}
pub fn is_log_enabled() -> bool {
    unsafe { LOG_ENABLED.load(Ordering::SeqCst) }
}
pub fn is_log_booted() -> bool {
    unsafe { LOG_BOOTED }
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if !is_log_enabled() || !is_log_booted() {
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
            "\u{1B}[{}m[{:>5}, HART{}, TID{} at {}ms] {}\u{1B}[0m",
            color,
            record.level(),
            get_hartid(),
            current_cpu()
                .task
                .as_ref()
                .map_or_else(|| 0, |task| task.tid()),
            get_time_ms(),
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
    set_log_booted();
    switch_log_on();
    info!("[init] log init success");
}

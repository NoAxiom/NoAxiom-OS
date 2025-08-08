use arch::{Arch, ArchTime};

use crate::{
    constant::time::MSEC_PER_SEC, include::time::TimeSpec, panic::kshutdown,
    time::gettime::get_time,
};

pub const LOG_BEGIN: TimeSpec = from_ms(0);
pub const LOG_END: TimeSpec = from_ms(60000);

pub const SHUTDOWN_WHEN_LOG_END: bool = true;

const fn from_ms(ms: usize) -> TimeSpec {
    TimeSpec {
        tv_sec: ms / 1000,
        tv_nsec: (ms % 1000) * 1_000_000,
    }
}

#[atomic_enum::atomic_enum]
enum LogState {
    Uninitialized,
    Off,
    On,
}

#[inline(always)]
pub fn logoff() {
    log::set_max_level(log::LevelFilter::Off);
}

#[inline(always)]
pub fn logon() {
    log::set_max_level(match option_env!("LOG") {
        Some("ERROR") => log::LevelFilter::Error,
        Some("WARN") => log::LevelFilter::Warn,
        Some("INFO") => log::LevelFilter::Info,
        Some("DEBUG") => log::LevelFilter::Debug,
        Some("TRACE") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Off,
    });
}

#[allow(unused)]
pub fn log_hook() {
    static mut LOG_STATE: AtomicLogState = AtomicLogState::new(LogState::Uninitialized);
    let current = get_time() / (Arch::get_freq() / MSEC_PER_SEC);

    match unsafe { LOG_STATE.load(core::sync::atomic::Ordering::SeqCst) } {
        LogState::Uninitialized => {
            logoff();
            unsafe {
                LOG_STATE.store(LogState::Off, core::sync::atomic::Ordering::SeqCst);
            };
        }
        LogState::Off => {
            if current >= LOG_BEGIN.into_ms() {
                println!("[kernel] Logging started at {} ms", current);
                logon();
                unsafe {
                    LOG_STATE.store(LogState::On, core::sync::atomic::Ordering::SeqCst);
                };
            }
        }
        LogState::On => {
            if current >= LOG_END.into_ms() {
                log::set_max_level(log::LevelFilter::Off);
                if SHUTDOWN_WHEN_LOG_END {
                    println!("[kernel] Time limit reached, shutting down...");
                    kshutdown();
                }
            }
        }
    }
}

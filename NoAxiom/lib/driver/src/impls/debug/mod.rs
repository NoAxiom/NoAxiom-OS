// temporary debug console for on-board debugs

mod debug_console;
mod debug_serial;

pub use debug_console::{debug_print, force_unlock_debug_console};
pub use debug_serial::DebugCharDev;

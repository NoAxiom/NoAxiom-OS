// temporary debug console for on-board debugs

mod debug_console;
mod debug_serial;

pub use debug_console::debug_print;
pub use debug_serial::DebugCharDev;

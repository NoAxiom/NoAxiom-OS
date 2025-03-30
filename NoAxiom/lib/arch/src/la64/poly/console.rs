// use devices::MAIN_UART;
use polyhal::debug_console::DebugConsole;

// sync get char
pub fn getchar() -> u8 {
    loop {
        // if let Some(c) = match MAIN_UART.try_get() {
        //     Some(uart) => uart.get(),
        //     None => DebugConsole::getchar(),
        // } {
        //     return c;
        // }
        if let Some(c) = DebugConsole::getchar() {
            return c;
        }
    }
}

pub fn putchar(c: u8) {
    // Use the main uart as much as possible.
    // let main_uart_inited = MAIN_UART.is_init();
    // match main_uart_inited {
    //     true => MAIN_UART.put(c),
    //     false => DebugConsole::putchar(c),
    // }
    DebugConsole::putchar(c);
}

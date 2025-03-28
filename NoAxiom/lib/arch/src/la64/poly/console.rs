use devices::MAIN_UART;

// sync get char
pub fn getchar() -> u8 {
    loop {
        if let Some(data) = MAIN_UART.try_get().unwrap().get() {
            return data;
        }
    }
}

pub fn putchar(c: u8) {
    MAIN_UART.try_get().unwrap().put(c);
}

pub mod sifive;
pub mod uart8250;

// use device_interface::{DeviceBase, UartDevice};
// use ksync::Mutex;

pub trait UartDriver: Send + Sync {
    fn init(&mut self);
    fn putchar(&mut self, byte: u8);
    fn getchar(&mut self) -> Option<u8>;
}
macro_rules! wait_for {
    ($cond:expr) => {{
        let mut timeout = 10000000;
        while !$cond && timeout > 0 {
            core::hint::spin_loop();
            timeout -= 1;
        }
    }};
}
pub(crate) use wait_for;

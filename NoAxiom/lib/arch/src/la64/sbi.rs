use super::{
    poly::console::{getchar, putchar},
    LA64,
};
use crate::ArchSbi;

impl ArchSbi for LA64 {
    fn console_getchar() -> usize {
        getchar() as usize
    }
    fn console_putchar(c: usize) {
        putchar(c as u8);
    }
    fn hart_start(_hartid: usize, _start_addr: usize) {
        unimplemented!()
    }
    fn shutdown() -> ! {
        unimplemented!()
    }
    fn send_ipi(_hartid: usize) {
        unimplemented!()
    }
    fn clear_ipi() {
        unimplemented!()
    }
}

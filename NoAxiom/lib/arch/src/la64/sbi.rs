use super::LA64;
use crate::ArchSbi;

impl ArchSbi for LA64 {
    fn console_getchar() -> usize {
        unimplemented!()
    }
    fn console_putchar(_c: usize) {
        unimplemented!()
    }
    fn hart_start(_hartid: usize, _start_addr: usize, _opaque: usize) {
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

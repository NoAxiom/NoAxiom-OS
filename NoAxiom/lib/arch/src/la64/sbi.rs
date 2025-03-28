use config::mm::KERNEL_STACK_SIZE;
use loongArch64::ipi::{csr_mail_send, send_ipi_single};

use super::LA64;
use crate::{la64::boot::BOOT_STACK, ArchSbi};

impl ArchSbi for LA64 {
    fn console_getchar() -> usize {
        unimplemented!()
    }
    fn console_putchar(_c: usize) {
        unimplemented!()
    }
    fn hart_start(hartid: usize, start_addr: usize) {
        let sp_addr = &BOOT_STACK as *const _ as usize + KERNEL_STACK_SIZE * hartid;
        csr_mail_send(start_addr as _, hartid, 0);
        csr_mail_send(sp_addr as _, hartid, 1);
        send_ipi_single(1, 1);
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

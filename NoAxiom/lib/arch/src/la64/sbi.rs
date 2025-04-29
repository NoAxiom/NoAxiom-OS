use config::mm::KERNEL_STACK_SIZE;
use loongArch64::ipi::{csr_mail_send, send_ipi_single};

use super::LA64;
use crate::{la64::boot::BOOT_STACK, ArchSbi};

impl ArchSbi for LA64 {
    fn hart_start(hartid: usize, start_addr: usize) {
        let sp_addr =
            &BOOT_STACK as *const _ as usize + KERNEL_STACK_SIZE * hartid + KERNEL_STACK_SIZE - 16;
        csr_mail_send(start_addr as _, hartid, 0);
        csr_mail_send(sp_addr as _, hartid, 1);
        send_ipi_single(hartid, 1);
    }
    fn send_ipi(hartid: usize) {
        send_ipi_single(hartid, 1);
    }
    fn clear_ipi() {
        unimplemented!()
    }
}

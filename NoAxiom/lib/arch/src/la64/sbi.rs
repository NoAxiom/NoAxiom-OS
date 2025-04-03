use config::mm::KERNEL_STACK_SIZE;
use loongArch64::ipi::{csr_mail_send, send_ipi_single};

use super::{
    poly::console::{getchar, putchar},
    LA64,
};
use crate::{
    la64::{boot::BOOT_STACK, memory::KERNEL_ADDR_OFFSET},
    Arch, ArchInt, ArchSbi,
};

impl ArchSbi for LA64 {
    fn console_getchar() -> usize {
        getchar() as usize
    }
    fn console_putchar(c: usize) {
        putchar(c as u8);
    }
    fn hart_start(hartid: usize, start_addr: usize) {
        let sp_addr =
            &BOOT_STACK as *const _ as usize + KERNEL_STACK_SIZE * hartid + KERNEL_STACK_SIZE - 16;
        csr_mail_send(start_addr as _, hartid, 0);
        csr_mail_send(sp_addr as _, hartid, 1);
        send_ipi_single(hartid, 1);
    }
    fn shutdown() -> ! {
        const HALT_ADDR: *mut u8 = (0x100E001C | KERNEL_ADDR_OFFSET) as *mut u8;

        #[inline]
        fn halt() {
            Arch::disable_interrupt();
            unsafe { loongArch64::asm::idle() }
        }

        // Shutdown the whole system, including all CPUs.
        unsafe { HALT_ADDR.write_volatile(0x34) };
        halt();
        loop {
            halt();
        }
    }
    fn send_ipi(hartid: usize) {
        send_ipi_single(hartid, 1);
    }
    fn clear_ipi() {
        unimplemented!()
    }
}

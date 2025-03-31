use config::mm::KERNEL_STACK_SIZE;
use loongArch64::ipi::{csr_mail_send, send_ipi_single};

use super::{
    poly::{
        console::{getchar, putchar},
        la64_dev_init,
    },
    LA64,
};
use crate::{la64::boot::BOOT_STACK, ArchSbi};

impl ArchSbi for LA64 {
    fn console_init() {
        la64_dev_init();
    }
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
        send_ipi_single(1, 1);
    }
    fn shutdown() -> ! {
        // [on_board] 电源管理模块设置为s5状态，软关机
        // unsafe { ((0x1FE27000 + 0x14) as *mut u32).write_volatile(0b1111 << 10) };
        loop {
            unsafe { loongArch64::asm::idle() }
        }
    }
    fn send_ipi(_hartid: usize) {
        unimplemented!()
    }
    fn clear_ipi() {
        unimplemented!()
    }
}

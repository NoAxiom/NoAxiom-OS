use fdt::Fdt;
use plic::{Mode, PLIC};
use spin::Once;

use crate::{config::arch::CPU_NUM, cpu::get_hartid, platform::platform_dtb_ptr};

pub static PLIC: Once<PLIC<CPU_NUM>> = Once::new();

// pub fn init_device() {
//     let dtb_ptr = platform_dtb_ptr();
//     let dtb = unsafe { Fdt::from_ptr(dtb_ptr as *const u8).unwrap() };
//     Probe::init(dtb_ptr);
// }

// pub fn init_plic(base_addr: usize) {
//     #[cfg(feature = "riscv_qemu")]
//     {
//         let privileges = [2; CPU_NUM];
//         let plic = PLIC::new(base_addr, privileges);
//         PLIC.call_once(|| plic);
//         info!("[init] PLIC init success, platform: QEMU");
//     }
// }

pub fn ext_int_handler() {
//     let plic = PLIC.get().unwrap();
//     let hart_id = get_hartid();
//     let irq = plic.claim(hart_id as u32, Mode::Supervisor);
//     let table = DEVICE_TABLE.lock();
//     let device = table
//         .get(&(irq as usize))
//         .or_else(|| panic!("no device for irq {}", irq))
//         .unwrap();
//     device.interrupt_handler();
//     plic.complete(hart_id as u32, Mode::Supervisor, irq);
//     let mut interrupts = INTERRUPT_RECORD.lock();
//     let value = interrupts.entry(irq as usize).or_insert(0);
//     *value += 1;
}

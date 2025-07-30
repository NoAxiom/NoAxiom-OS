// //! Adapted from MankorOS

// pub mod uart8250;

// use alloc::{boxed::Box, collections::VecDeque, string::ToString, sync::Arc};
// use core::{
//     cell::UnsafeCell,
//     cmp,
//     fmt::{self, Debug, Write},
//     task::Waker,
// };

// use async_trait::async_trait;
// use async_utils::{block_on, get_waker};
// use config::{board::UART_BUF_LEN, mm::VIRT_RAM_OFFSET};
// use device_core::{DevId, Device, DeviceMajor, DeviceMeta, DeviceType};
// use fdt::Fdt;
// use macro_utils::with_methods;
// use memory::pte::PTEFlags;
// use ring_buffer::RingBuffer;
// use sync::mutex::SpinNoIrqLock;

// use super::CharDevice;
// use crate::{kernel_page_table_mut, manager::DeviceManager, println,
// serial::uart8250::Uart};

// pub static mut UART0: Option<Arc<Serial>> = None;

// trait UartDriver: Send + Sync {
//     fn init(&mut self);
//     fn putc(&mut self, byte: u8);
//     fn getc(&mut self) -> u8;
//     fn poll_in(&self) -> bool;
//     fn poll_out(&self) -> bool;
// }

// pub struct Serial {
//     meta: DeviceMeta,
//     uart: UnsafeCell<Box<dyn UartDriver>>,
//     inner: SpinNoIrqLock<SerialInner>,
// }

// pub struct SerialInner {
//     read_buf: RingBuffer,
//     /// Hold wakers of pollin tasks.
//     pollin_queue: VecDeque<Waker>,
// }

// unsafe impl Send for Serial {}
// unsafe impl Sync for Serial {}

// impl Serial {
//     fn new(mmio_base: usize, mmio_size: usize, irq_no: usize, driver: Box<dyn
// UartDriver>) -> Self {         let meta = DeviceMeta {
//             dev_id: DevId {
//                 major: DeviceMajor::Serial,
//                 minor: 0,
//             },
//             name: "serial".to_string(),
//             mmio_base,
//             mmio_size,
//             irq_no: Some(irq_no),
//             dtype: DeviceType::Char,
//         };

//         Self {
//             meta,
//             uart: UnsafeCell::new(driver),
//             inner: SpinNoIrqLock::new(SerialInner {
//                 read_buf: RingBuffer::new(UART_BUF_LEN),
//                 pollin_queue: VecDeque::new(),
//             }),
//         }
//     }

//     fn uart(&self) -> &mut Box<dyn UartDriver> {
//         unsafe { &mut *self.uart.get() }
//     }

//     with_methods!(inner: SerialInner);
// }

// impl fmt::Debug for Serial {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Serial")
//     }
// }

// impl Device for Serial {
//     fn meta(&self) -> &DeviceMeta {
//         &self.meta
//     }

//     fn init(&self) {
//         unsafe { &mut *self.uart.get() }.as_mut().init()
//     }

//     fn handle_irq(&self) {
//         let uart = self.uart();
//         self.with_mut_inner(|inner| {
//             while uart.poll_in() {
//                 let byte = uart.getc();
//                 log::info!(
//                     "Serial interrupt handler got byte: {}",
//                     core::str::from_utf8(&[byte]).unwrap()
//                 );
//                 if inner.read_buf.enqueue(byte).is_none() {
//                     break;
//                 }
//             }
//             // Round Robin
//             if let Some(waiting) = inner.pollin_queue.pop_front() {
//                 waiting.wake();
//             }
//         });
//     }
// }

// #[async_trait]
// impl CharDevice for Serial {
//     async fn read(&self, buf: &mut [u8]) -> usize {
//         let mut len = 0;
//         self.with_mut_inner(|inner| {
//             len = inner.read_buf.read(buf);
//         });
//         let uart = self.uart();
//         while uart.poll_in() && len < buf.len() {
//             let c = uart.getc();
//             buf[len] = c;
//             len += 1;
//         }
//         len
//     }

//     async fn write(&self, buf: &[u8]) -> usize {
//         let uart = self.uart();
//         for &c in buf {
//             uart.putc(c)
//         }
//         buf.len()
//     }

//     async fn poll_in(&self) -> bool {
//         let uart = self.uart();
//         let waker = get_waker().await;
//         self.with_mut_inner(|inner| {
//             if uart.poll_in() || !inner.read_buf.is_empty() {
//                 return true;
//             }
//             inner.pollin_queue.push_back(waker);
//             false
//         })
//     }

//     // TODO:
//     async fn poll_out(&self) -> bool {
//         true
//     }
// }

// impl DeviceManager {
//     pub fn probe_char_device(&mut self, root: &Fdt) {
//         let chosen = root.chosen();
//         // Serial
//         let mut stdout = chosen.stdout();
//         if stdout.is_none() {
//             println!("Non-standard stdout device, trying to workaround");
//             let chosen = root.find_node("/chosen").expect("No chosen node");
//             let stdout_path = chosen
//                 .properties()
//                 .find(|n| n.name == "stdout-path")
//                 .and_then(|n| {
//                     let bytes = unsafe {
//                         core::slice::from_raw_parts_mut(
//                             (n.value.as_ptr()) as *mut u8,
//                             n.value.len(),
//                         )
//                     };
//                     let mut len = 0;
//                     for byte in bytes.iter() {
//                         if *byte == b':' {
//                             return
// core::str::from_utf8(&n.value[..len]).ok();                         }
//                         len += 1;
//                     }
//                     core::str::from_utf8(&n.value[..n.value.len() - 1]).ok()
//                 })
//                 .unwrap();
//             println!("Searching stdout: {}", stdout_path);
//             stdout = root.find_node(stdout_path);
//         }
//         if stdout.is_none() {
//             println!("Unable to parse /chosen, choosing first serial
// device");             stdout = root.find_compatible(&[
//                 "ns16550a",
//                 "snps,dw-apb-uart", // C910, VF2
//                 "sifive,uart0",     // sifive_u QEMU (FU540)
//             ])
//         }
//         let stdout = stdout.expect("Still unable to get stdout device");
//         println!("Stdout: {}", stdout.name);

//         let serial = probe_serial_console(&stdout);

//         self.devices.insert(serial.dev_id(), Arc::new(serial));
//     }
// }

// /// This guarantees to return a Serial device
// /// The device is not initialized yet
// fn probe_serial_console(stdout: &fdt::node::FdtNode) -> Serial {
//     let reg = stdout.reg().unwrap().next().unwrap();
//     let base_paddr = reg.starting_address as usize;
//     let size = reg.size.unwrap();
//     let base_vaddr = base_paddr + VIRT_RAM_OFFSET;
//     let irq_number =
// stdout.property("interrupts").unwrap().as_usize().unwrap();     log::info!("
// IRQ number: {}", irq_number);     kernel_page_table_mut().ioremap(base_paddr,
// size, PTEFlags::R | PTEFlags::W);     let first_compatible =
// stdout.compatible().unwrap().first();     match first_compatible {
//         "ns16550a" | "snps,dw-apb-uart" => {
//             // VisionFive 2 (FU740)
//             // virt QEMU

//             // Parse clock frequency
//             let freq_raw = stdout
//                 .property("clock-frequency")
//                 .expect("No clock-frequency property of stdout serial
// device")                 .as_usize()
//                 .expect("Parse clock-frequency to usize failed");
//             let mut reg_io_width = 1;
//             if let Some(reg_io_width_raw) = stdout.property("reg-io-width") {
//                 reg_io_width = reg_io_width_raw
//                     .as_usize()
//                     .expect("Parse reg-io-width to usize failed");
//             }
//             let mut reg_shift = 0;
//             if let Some(reg_shift_raw) = stdout.property("reg-shift") {
//                 reg_shift = reg_shift_raw
//                     .as_usize()
//                     .expect("Parse reg-shift to usize failed");
//             }
//             log::info!("uart: base_paddr:{base_paddr:#x}, size:{size:#x},
// reg_io_width:{reg_io_width}, reg_shift:{reg_shift}");

//             let uart = unsafe {
//                 Uart::new(
//                     base_vaddr,
//                     freq_raw,
//                     115200,
//                     reg_io_width,
//                     reg_shift,
//                     first_compatible == "snps,dw-apb-uart",
//                 )
//             };
//             Serial::new(base_paddr, size, irq_number, Box::new(uart))
//         }
//         _ => panic!("Unsupported serial console"),
//     }
// }

// pub mod ns16550a;

use alloc::{boxed::Box, collections::vec_deque::VecDeque, sync::Arc};
use core::{
    cell::UnsafeCell,
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Poll, Waker},
};

use log::{info, warn};
use ringbuffer::RingBuffer;
use spin::Once;

use super::{CharDevice, Device};
use crate::{
    device::{ADevResult, DeviceType},
    driver::uart::UartDriver,
    sync::mutex::SpinMutex, nix::result::Errno,
};

pub static UART_DEVICE: Once<Arc<Serial>> = Once::new();

pub fn init_uart_device(uart: Arc<Serial>) {
    uart.init();
    UART_DEVICE.call_once(|| uart);
}
pub struct Serial {
    inner: UnsafeCell<Box<dyn UartDriver>>,
    buffer: SpinMutex<ringbuffer::ConstGenericRingBuffer<u8, 512>>, // Hard-coded buffer size
    waiting: SpinMutex<VecDeque<Waker>>,
    base_address: usize,
    size: usize,
    interrupt_number: usize,
}

unsafe impl Send for Serial {}
unsafe impl Sync for Serial {}

impl Serial {
    pub fn new(
        base_address: usize,
        size: usize,
        interrupt_number: usize,
        driver: Box<dyn UartDriver>,
    ) -> Self {
        Self {
            inner: UnsafeCell::new(driver),
            buffer: SpinMutex::new(ringbuffer::ConstGenericRingBuffer::new()),
            base_address,
            size,
            interrupt_number,
            waiting: SpinMutex::new(VecDeque::new()),
        }
    }
    pub fn read<'a>(&'a self, buf: Pin<&'a mut [u8]>) -> ADevResult<isize> {
        Box::pin(SerialReadFuture { buf, inner: self })
    }

    pub fn write(&self, buf: &[u8]) -> Result<(), Errno> {
        for byte in buf {
            unsafe { &mut *self.inner.get() }.as_mut().putchar(*byte)
        }
        Ok(())
    }
}

impl Debug for Serial {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Serial")
    }
}

impl Device for Serial {
    fn name(&self) -> &str {
        "Uart"
    }

    fn dev_type(&self) -> DeviceType {
        DeviceType::Char
    }

    fn mmio_base(&self) -> usize {
        self.base_address
    }

    fn mmio_size(&self) -> usize {
        self.size
    }

    fn interrupt_number(&self) -> Option<usize> {
        Some(self.interrupt_number)
    }

    fn interrupt_handler(&self) {
        let byte = unsafe { &mut *self.inner.get() }.as_mut().getchar();
        if let Some(b) = byte {
            info!(
                "Serial interrupt handler got byte: {}",
                core::str::from_utf8(&[b]).unwrap()
            );
            self.buffer.lock().enqueue(b);
            // Round Robin
            if let Some(waiting) = self.waiting.lock().pop_front() {
                waiting.wake();
            }
        } else {
            warn!("Serial interrupt handler got no byte");
        }
    }

    fn init(&self) {
        unsafe { &mut *self.inner.get() }.as_mut().init()
    }

    fn driver(&self) -> Option<Arc<dyn crate::driver::Driver>> {
        todo!()
    }

    fn set_driver(&self, _driver: Option<alloc::sync::Weak<dyn crate::driver::Driver>>) {
        todo!()
    }

    fn is_dead(&self) -> bool {
        todo!()
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn crate::device::block::BlockDevice>> {
        None
    }

    fn as_char(self: Arc<Self>) -> Option<Arc<dyn CharDevice>> {
        Some(self)
    }
}
pub struct SerialReadFuture<'a> {
    buf: Pin<&'a mut [u8]>,
    inner: &'a Serial,
}

impl Future for SerialReadFuture<'_> {
    type Output = Result<isize, Errno>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if let Some(char) = this.inner.buffer.lock().dequeue() {
            this.buf[0] = char;
            Poll::Ready(Ok(1))
        } else {
            // Push itself to the waiting queue

            this.inner.waiting.lock().push_back(cx.waker().clone());
            Poll::Pending
        }
    }
}
impl CharDevice for Serial {
    fn read<'a>(&'a self, buf: Pin<&'a mut [u8]>) -> ADevResult<isize> {
        self.read(buf)
    }

    fn write(&self, buf: &[u8]) -> Result<(), Errno> {
        self.write(buf)
    }
}

use alloc::collections::VecDeque;
use core::any::Any;

// use spin::Mutex;
use crate::driver::virtio_drivers2::{input::VirtIOInput, VirtIOHeader};

type Mutex<T> = ksync::mutex::SpinLock<T>;

pub trait InputDevice: Send + Sync + Any {
    fn read_event(&self) -> u64;
    fn is_empty(&self) -> bool;
}

// unsafe impl Send for VirtIOGpuWrapper {}
// unsafe impl Sync for VirtIOGpuWrapper {}

pub struct VirtIOInputWrapper {
    input_device: Mutex<VirtIOInput<'static>>,
    events: Mutex<VecDeque<u64>>,
}
impl VirtIOInputWrapper {
    pub fn new(addr: usize) -> Self {
        unsafe {
            let virtio = VirtIOInput::new(&mut *(addr as *mut VirtIOHeader)).unwrap();
            Self {
                input_device: Mutex::new(virtio),
                events: Mutex::new(VecDeque::new()),
            }
        }
    }
}
impl InputDevice for VirtIOInputWrapper {
    fn is_empty(&self) -> bool {
        self.events.lock().is_empty()
    }

    fn read_event(&self) -> u64 {
        let mut input_device = self.input_device.lock();
        let mut events = self.events.lock();
        while let Some(event) = input_device.pop_pending_event() {
            let result =
                (event.event_type as u64) << 48 | (event.code as u64) << 32 | (event.value) as u64;
            events.push_back(result);
        }
        // merge mouse move events
        let mut shift_x: u32 = 0;
        let mut shift_y: u32 = 0;
        while let Some(e) = events.front() {
            if *e == 0 {
                events.pop_front();
                continue;
            }
            let mut v = e.clone() as u64;
            let value = v as u32;
            v >>= 32;
            let code = v as u16;
            v >>= 16;
            let event_type = v as u16;
            if (event_type == 2) || (event_type == 3) {
                if code == 0 {
                    shift_x += value;
                } else if code == 1 {
                    shift_y += value;
                }
            } else {
                break;
            }
            events.pop_front();
        }
        if shift_x != 0 {
            let v = (2u64 << 48) | (0u64 << 32) | (shift_x as u32 as u64);
            events.push_back(v);
        }
        if shift_y != 0 {
            let v = (2u64 << 48) | (1u64 << 32) | (shift_y as u32 as u64);
            events.push_back(v);
        }
        if let Some(event) = events.pop_front() {
            return event;
        } else {
            0
        }
    }
}

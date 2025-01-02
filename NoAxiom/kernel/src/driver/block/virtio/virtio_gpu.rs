use core::any::Any;

type Mutex<T> = ksync::mutex::SpinLock<T>;

use crate::{
    config::mm::KERNEL_ADDR_OFFSET,
    driver::virtio_drivers2::{gpu::VirtIOGpu, header::VirtIOHeader},
};

const VIRTIO7: usize = 0x10007000 + KERNEL_ADDR_OFFSET;

pub trait GpuDevice: Send + Sync + Any {
    fn update_cursor(&self);
    fn get_framebuffer(&self) -> &mut [u8];
    fn flush(&self);
}

// unsafe impl Send for VirtIOGpuWrapper {}
// unsafe impl Sync for VirtIOGpuWrapper {}

pub struct VirtIOGpuWrapper {
    gpu: Mutex<VirtIOGpu<'static>>,
    fb: &'static [u8],
}
impl VirtIOGpuWrapper {
    pub fn new() -> Self {
        unsafe {
            let mut virtio = VirtIOGpu::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();
            let fbuffer = virtio.setup_framebuffer().unwrap();
            let len = fbuffer.len();
            let ptr = fbuffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);
            Self {
                gpu: Mutex::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    fn flush(&self) {
        self.gpu.lock().flush().unwrap();
    }
    fn get_framebuffer(&self) -> &mut [u8] {
        // let (x, y) = self.gpu.lock().resolution();
        // println!("RESOLUTION {:?},{:?}", x, y);
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }
    fn update_cursor(&self) {}
}

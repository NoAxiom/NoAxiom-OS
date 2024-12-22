use alloc::sync::Arc;

pub use visionfive2_sd::Vf2SdDriver;
use visionfive2_sd::{SDIo, SleepOps};

use super::BlockDriver;
use crate::{
    config::{errno::Errno, mm::KERNEL_ADDR_OFFSET},
    devices::{Device, IdTable},
    drivers::{
        Driver,
        Probe::{ProbeInfo, PROBE},
    },
    sync::mutex::Mutex,
    time::gettime::get_time,
};

fn probe() -> Option<ProbeInfo> {
    if cfg!(any(feature = "vf2")) {
        return None;
    }
    PROBE.get().unwrap().probe_sdio()
}
// time need to fix
pub fn sleep_ms(ms: usize) {
    let start = get_time();
    while get_time() - start < ms * 400_0000 / 1000 {
        core::hint::spin_loop();
    }
}

pub fn sleep_ms_until(ms: usize, mut f: impl FnMut() -> bool) {
    let start = get_time();
    while get_time() - start < ms * 400_0000 / 1000 {
        if f() {
            return;
        }
        core::hint::spin_loop();
    }
}
// unsafe impl Send for SdIoImpl {}
// unsafe impl Sync for SdIoImpl {}
pub struct SdIoImpl;
pub const SDIO_BASE: usize = 0x16020000 + KERNEL_ADDR_OFFSET; // need to fix
impl SDIo for SdIoImpl {
    fn read_data_at(&self, offset: usize) -> u64 {
        let addr = (SDIO_BASE + offset) as *mut u64;
        unsafe { addr.read_volatile() }
    }
    fn read_reg_at(&self, offset: usize) -> u32 {
        let addr = (SDIO_BASE + offset) as *mut u32;
        unsafe { addr.read_volatile() }
    }
    fn write_data_at(&mut self, offset: usize, val: u64) {
        let addr = (SDIO_BASE + offset) as *mut u64;
        unsafe { addr.write_volatile(val) }
    }
    fn write_reg_at(&mut self, offset: usize, val: u32) {
        let addr = (SDIO_BASE + offset) as *mut u32;
        unsafe { addr.write_volatile(val) }
    }
}
// unsafe impl Send for SleepOpsImpl {}
// unsafe impl Sync for SleepOpsImpl {}
pub struct SleepOpsImpl;

impl SleepOps for SleepOpsImpl {
    fn sleep_ms(ms: usize) {
        sleep_ms(ms)
    }
    fn sleep_ms_until(ms: usize, f: impl FnMut() -> bool) {
        sleep_ms_until(ms, f)
    }
}
pub struct VF2SDDriver {
    driver: Mutex<Vf2SdDriver<SdIoImpl, SleepOpsImpl>>,
}

impl VF2SDDriver {
    pub fn new(sd: Vf2SdDriver<SdIoImpl, SleepOpsImpl>) -> Self {
        // 8.13 M
        let ret = VF2SDDriver {
            driver: Mutex::new(sd),
        };
        ret.init();
        ret
    }
    pub fn init(&self) {
        self.driver.lock().init();
    }
}
impl Driver for VF2SDDriver {
    fn id_table(&self) -> Option<IdTable> {
        None
    }

    fn as_blk(self: Arc<Self>) -> Option<Arc<dyn BlockDriver>> {
        Some(self)
    }
}
impl BlockDriver for VF2SDDriver {
    fn handle_irq(&self) {
        unimplemented!()
    }
    fn read_block(&self, blk_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        // println!("read_block");
        self.driver.lock().read_block(blk_id, buf);
        Ok(())
    }

    fn write_block(&self, blk_id: usize, buf: &[u8]) -> Result<(), Errno> {
        self.driver.lock().write_block(blk_id, buf);
        Ok(())
    }

    fn read_async_block(&self, blk_id: usize, buf: &mut [u8]) -> Result<(), Errno> {
        todo!()
    }

    fn write_async_block(&self, blk_id: usize, buf: &[u8]) -> Result<(), Errno> {
        todo!()
    }
    // need to fix
    fn size(&self) -> usize {
        32 * 1024 * 1024 * 1024 / 512
    }

    fn flush(&self) -> Result<(), Errno> {
        todo!()
    }
}

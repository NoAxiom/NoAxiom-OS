use arch::{consts::IO_ADDR_OFFSET, Arch, ArchTime};
use ksync::AsyncMutex;

use crate::{
    basic::{BlockDeviceType, Device, DeviceType},
    block::{
        vf2_sdcard::{
            register::{get_sdio_pbase, set_sdio_base},
            SDIo, SleepOps, Vf2SdDriver,
        },
        BlockDevice,
    },
    DevResult,
};

// time need to fix
pub fn sleep_ms(ms: usize) {
    let start = get_time_ms();
    while get_time_ms() - start < ms {
        core::hint::spin_loop();
    }
}

pub fn sleep_ms_until(ms: usize, mut f: impl FnMut() -> bool) {
    let start = get_time_ms();
    while get_time_ms() - start < ms {
        if f() {
            return;
        }
        core::hint::spin_loop();
    }
}

pub struct SdIoImpl;
impl SDIo for SdIoImpl {
    fn read_data_at(&self, offset: usize) -> u64 {
        let addr = (get_sdio_pbase() + offset) as *mut u64;
        unsafe { addr.read_volatile() }
    }
    fn read_reg_at(&self, offset: usize) -> u32 {
        let addr = (get_sdio_pbase() + offset) as *mut u32;
        unsafe { addr.read_volatile() }
    }
    fn write_data_at(&mut self, offset: usize, val: u64) {
        let addr = (get_sdio_pbase() + offset) as *mut u64;
        unsafe { addr.write_volatile(val) }
    }
    fn write_reg_at(&mut self, offset: usize, val: u32) {
        let addr = (get_sdio_pbase() + offset) as *mut u32;
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

pub struct VF2SdcardDevice {
    inner: AsyncMutex<Vf2SdDriver<SdIoImpl, SleepOpsImpl>>,
}

impl VF2SdcardDevice {
    // compatible: starfive,jh7110-sdio
    pub fn new(base_addr: usize) -> Self {
        // 8.13 M
        set_sdio_base(base_addr);
        let sd = Vf2SdDriver::<_, SleepOpsImpl>::new(SdIoImpl);
        let ret = VF2SdcardDevice {
            inner: AsyncMutex::new(sd),
        };
        ret.init();
        ret
    }
    pub fn init(&self) {
        self.inner.spin_lock().init();
    }
}

impl Device for VF2SdcardDevice {
    fn device_name(&self) -> &'static str {
        "VF2SdcardDevice"
    }
    fn device_type(&self) -> &'static DeviceType {
        &DeviceType::Block(BlockDeviceType::PhysRV)
    }
}

#[async_trait::async_trait]
impl BlockDevice for VF2SdcardDevice {
    fn sync_read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        self.inner.spin_lock().read_block(id, buf);
        Ok(buf.len())
    }
    fn sync_write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        self.inner.spin_lock().write_block(id, buf);
        Ok(buf.len())
    }
    // async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
    //     self.inner.lock().await.read_block(id, buf);
    //     Ok(buf.len())
    // }
    // async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
    //     self.inner.lock().await.write_block(id, buf);
    //     Ok(buf.len())
    // }
}

pub fn get_time_ms() -> usize {
    const MSEC_PER_SEC: usize = 1000;
    arch::Arch::get_time() / (Arch::get_freq() / MSEC_PER_SEC)
}

use alloc::boxed::Box;
use core::task::Waker;

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use kfuture::{suspend::SuspendFuture, take_waker::TakeWakerFuture};
use ksync::{async_mutex::AsyncMutex, cell::SyncUnsafeCell};
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, RespStatus, VirtIOBlk},
    transport::Transport,
};

use crate::{
    devices::{
        block::BlockDevice,
        hal::{dev_err, VirtioHalImpl},
        DevResult,
    },
    plic::{disable_blk_irq, enable_blk_irq},
};

struct VirioBlkInner<T: Transport> {
    blk: VirtIOBlk<VirtioHalImpl, T>,
}

impl<T: Transport> VirioBlkInner<T> {
    fn sync_read(&mut self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        self.blk.read_blocks(id, buf).map_err(dev_err)?;
        Ok(buf.len())
    }
    fn sync_write(&mut self, id: usize, buf: &[u8]) -> DevResult<usize> {
        self.blk.write_blocks(id, buf).map_err(dev_err)?;
        Ok(buf.len())
    }
    fn read_req(
        &mut self,
        id: usize,
        request: &mut BlkReq,
        buffer: &mut [u8],
        response: &mut BlkResp,
    ) -> DevResult<u16> {
        unsafe { self.blk.read_blocks_nb(id, request, buffer, response) }.map_err(dev_err)
    }
    fn read_response(
        &mut self,
        token: u16,
        request: &BlkReq,
        buffer: &mut [u8],
        response: &mut BlkResp,
    ) -> DevResult<()> {
        unsafe {
            self.blk
                .complete_read_blocks(token, request, buffer, response)
        }
        .map_err(dev_err)?;
        assert_eq!(response.status(), RespStatus::OK);
        Ok(())
    }
    fn write_req(
        &mut self,
        id: usize,
        request: &mut BlkReq,
        buffer: &[u8],
        response: &mut BlkResp,
    ) -> DevResult<u16> {
        unsafe { self.blk.write_blocks_nb(id, request, buffer, response) }.map_err(dev_err)
    }
    fn write_response(
        &mut self,
        token: u16,
        request: &BlkReq,
        buffer: &[u8],
        response: &mut BlkResp,
    ) -> DevResult<()> {
        unsafe {
            self.blk
                .complete_write_blocks(token, request, buffer, response)
        }
        .map_err(dev_err)?;
        assert_eq!(response.status(), RespStatus::OK);
        Ok(())
    }
}

pub struct VirtioBlockDevice<T: Transport> {
    inner: AsyncMutex<VirioBlkInner<T>>,
    waker: SyncUnsafeCell<Option<Waker>>,
    irq: SyncUnsafeCell<bool>,
}

impl<T: Transport> VirtioBlockDevice<T> {
    /// Initializes the VirtIO block device.
    pub fn new(transport: T) -> Self {
        Self {
            inner: AsyncMutex::new(VirioBlkInner {
                blk: VirtIOBlk::new(transport).expect("Failed to create VirtIOBlk"),
            }),
            waker: SyncUnsafeCell::new(None),
            irq: SyncUnsafeCell::new(true),
        }
    }
    fn save(&self, waker: Waker) {
        assert!(self.waker.as_ref_mut().replace(waker).is_none());
    }
    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref_mut().take() {
            waker.wake();
        } else {
            panic!("No waker to wake up!");
        }
    }
    pub fn disable_irq(&self) {
        let irq = self.irq.get();
        if unsafe { *irq } {
            unsafe { *irq = false };
            disable_blk_irq();
        }
    }
    pub fn enable_irq(&self) {
        let irq = self.irq.get();
        if !unsafe { *irq } {
            unsafe { *irq = true };
            enable_blk_irq();
        }
    }
}

#[async_trait]
impl<T: Transport + Send> BlockDevice for VirtioBlockDevice<T> {
    fn device_name(&self) -> &'static str {
        "VirtIOBlockWrapper"
    }
    fn handle_interrupt(&self) -> DevResult<()> {
        self.wake();
        Ok(())
    }
    fn sync_read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        let mut inner = self.inner.spin_lock();
        // self.disable_irq();
        let res = inner.sync_read(id, buf);
        // self.enable_irq();
        res
    }
    fn sync_write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        let mut inner = self.inner.spin_lock();
        // self.disable_irq();
        let res = inner.sync_write(id, buf);
        // self.enable_irq();
        res
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        if buf.len() <= 2048 {
            return Ok(self.sync_read(id, buf).unwrap());
        }
        let mut inner = self.inner.lock().await;
        let mut request = BlkReq::default();
        let mut response = BlkResp::default();

        Arch::disable_external_interrupt();
        let token = inner
            .read_req(id, &mut request, buf, &mut response)
            .unwrap();
        self.save(TakeWakerFuture.await);
        SuspendFuture::new().await; // Wait for an interrupt to tell us that the request completed...
        inner
            .read_response(token, &request, buf, &mut response)
            .unwrap();

        Ok(buf.len())
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        if buf.len() <= 2048 {
            return Ok(self.sync_write(id, buf).unwrap());
        }
        let mut inner = self.inner.lock().await;
        let mut request = BlkReq::default();
        let mut response = BlkResp::default();

        Arch::disable_external_interrupt();
        let token = inner
            .write_req(id, &mut request, buf, &mut response)
            .unwrap();
        self.save(TakeWakerFuture.await);
        SuspendFuture::new().await; // Wait for an interrupt to tell us that the request completed...
        inner
            .write_response(token, &request, buf, &mut response)
            .unwrap();

        Ok(buf.len())
    }
}

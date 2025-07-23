use alloc::{boxed::Box, sync::Arc};
use core::{
    future::Future,
    ptr::NonNull,
    task::{Poll, Waker},
};

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use ksync::{
    async_mutex::{AsyncMutex, AsyncMutexGuard},
    cell::SyncUnsafeCell,
    Once,
};
use platform::dtb::basic::dtb_info;
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, RespStatus, VirtIOBlk},
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        pci::PciTransport,
        Transport,
    },
};

use crate::{
    device_cast,
    devices::{
        basic::Device,
        block::BlockDevice,
        hal::{dev_err, VirtioHalImpl},
        DevResult,
    },
    register_blk_dev,
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
}

impl<T: Transport> VirtioBlockDevice<T> {
    /// Initializes the VirtIO block device.
    pub fn new(transport: T) -> Self {
        Self {
            inner: AsyncMutex::new(VirioBlkInner {
                blk: VirtIOBlk::new(transport).expect("Failed to create VirtIOBlk"),
            }),
            waker: SyncUnsafeCell::new(None),
        }
    }
    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref_mut().take() {
            log::error!("wake waker");
            waker.wake();
        } else {
            log::error!("No waker to wake up!");
        }
    }
}

impl<T: Transport> Device for VirtioBlockDevice<T> {
    fn device_name(&self) -> &'static str {
        "VirtIOBlockWrapper"
    }
}

#[async_trait]
impl<T: Transport + Send> BlockDevice for VirtioBlockDevice<T> {
    fn handle_interrupt(&self) -> DevResult<()> {
        self.wake();
        Ok(())
    }
    fn sync_read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        let mut inner = self.inner.spin_lock();
        let res = inner.sync_read(id, buf);
        res
    }
    fn sync_write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        let mut inner = self.inner.spin_lock();
        let res = inner.sync_write(id, buf);
        res
    }
    async fn read(&self, id: usize, buf: &mut [u8]) -> DevResult<usize> {
        if buf.len() <= 2048 {
            return self.sync_read(id, buf);
        }
        let inner = self.inner.lock().await;
        let request = BlkReq::default();
        let response = BlkResp::default();

        ReadFuture::new(id, request, buf, response, inner, self.waker.as_ref_mut()).await
    }
    async fn write(&self, id: usize, buf: &[u8]) -> DevResult<usize> {
        if buf.len() <= 2048 {
            return self.sync_write(id, buf);
        }
        let inner = self.inner.lock().await;
        let request = BlkReq::default();
        let response = BlkResp::default();

        WriteFuture::new(id, request, buf, response, inner, self.waker.as_ref_mut()).await
    }
}

struct ReadFuture<'a, T: Transport> {
    id: usize,
    request: BlkReq,
    buffer: &'a mut [u8],
    response: BlkResp,
    sent: bool,
    token: u16,
    guard: AsyncMutexGuard<'a, VirioBlkInner<T>>,
    waker: &'a mut Option<Waker>,
}

impl<'a, T: Transport> ReadFuture<'a, T> {
    fn new(
        id: usize,
        request: BlkReq,
        buffer: &'a mut [u8],
        response: BlkResp,
        guard: AsyncMutexGuard<'a, VirioBlkInner<T>>,
        waker: &'a mut Option<Waker>,
    ) -> Self {
        Self {
            id,
            request,
            buffer,
            response,
            sent: false,
            token: 0,
            guard,
            waker,
        }
    }
}

impl<'a, T: Transport> Future for ReadFuture<'a, T> {
    type Output = DevResult<usize>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.get_mut();
        if !this.sent {
            this.sent = true;
            *this.waker = Some(cx.waker().clone());
            Arch::disable_external_interrupt();
            this.token = this
                .guard
                .read_req(this.id, &mut this.request, this.buffer, &mut this.response)
                .unwrap();
            return Poll::Pending;
        }

        if let Ok(_) =
            this.guard
                .read_response(this.token, &this.request, this.buffer, &mut this.response)
        {
            Poll::Ready(Ok(this.buffer.len()))
        } else {
            *this.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

struct WriteFuture<'a, T: Transport> {
    id: usize,
    request: BlkReq,
    buffer: &'a [u8],
    response: BlkResp,
    sent: bool,
    token: u16,
    guard: AsyncMutexGuard<'a, VirioBlkInner<T>>,
    waker: &'a mut Option<Waker>,
}

impl<'a, T: Transport> WriteFuture<'a, T> {
    fn new(
        id: usize,
        request: BlkReq,
        buffer: &'a [u8],
        response: BlkResp,
        guard: AsyncMutexGuard<'a, VirioBlkInner<T>>,
        waker: &'a mut Option<Waker>,
    ) -> Self {
        Self {
            id,
            request,
            buffer,
            response,
            sent: false,
            token: 0,
            guard,
            waker,
        }
    }
}

impl<'a, T: Transport> Future for WriteFuture<'a, T> {
    type Output = DevResult<usize>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.get_mut();
        if !this.sent {
            this.sent = true;
            *this.waker = Some(cx.waker().clone());
            Arch::disable_external_interrupt();
            this.token = this
                .guard
                .write_req(this.id, &mut this.request, this.buffer, &mut this.response)
                .unwrap();
            // HERE cannot receive the interrupt
            return Poll::Pending;
        }

        if let Ok(_) =
            this.guard
                .write_response(this.token, &this.request, this.buffer, &mut this.response)
        {
            Poll::Ready(Ok(this.buffer.len()))
        } else {
            *this.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// ====== MMIO defined device init ======
static MMIO_DEV: Once<VirtioBlockDevice<MmioTransport>> = Once::new();
fn realize_virtio_block_device_mmio() -> Option<VirtioBlockDevice<MmioTransport>> {
    let dtb_info = dtb_info();
    if dtb_info.virtio.mmio_regions.is_empty() {
        return None;
    }

    let (addr, size) = dtb_info.virtio.mmio_regions[0].simplified();
    log::info!("[driver] probe virtio wrapper at {:#x}", addr);
    let addr = addr | arch::consts::KERNEL_ADDR_OFFSET;
    let header = NonNull::new(addr as *mut VirtIOHeader).unwrap();
    let transport = unsafe { MmioTransport::new(header, size).unwrap() };

    Some(VirtioBlockDevice::new(transport))
}
pub(crate) fn block_mmio_init() {
    if let Some(dev) = realize_virtio_block_device_mmio() {
        MMIO_DEV.call_once(|| dev);
        register_blk_dev(device_cast!(MMIO_DEV, BlockDevice));
    } else {
        log::warn!("[driver] no virtio block device found in MMIO regions");
    }
}

// ====== PCI defined device init ======
static PCI_DEV: Once<VirtioBlockDevice<PciTransport>> = Once::new();
pub(crate) fn register_virtio_block_device_pci(dev: VirtioBlockDevice<PciTransport>) {
    PCI_DEV.call_once(|| dev);
    register_blk_dev(device_cast!(PCI_DEV, BlockDevice));
}

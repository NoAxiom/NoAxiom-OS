use alloc::boxed::Box;
use core::{
    future::Future,
    task::{Poll, Waker},
};

use arch::{Arch, ArchInt};
use async_trait::async_trait;
use ksync::{
    async_mutex::{AsyncMutex, AsyncMutexGuard},
    cell::SyncUnsafeCell,
};
use virtio_drivers::{
    device::blk::{BlkReq, BlkResp, RespStatus, VirtIOBlk},
    transport::Transport,
};

use crate::devices::{
    block::BlockDevice,
    hal::{dev_err, VirtioHalImpl},
    DevResult,
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

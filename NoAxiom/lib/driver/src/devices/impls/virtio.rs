use alloc::vec::Vec;
use core::ptr::NonNull;

use arch::consts::KERNEL_ADDR_OFFSET;
use include::errno::Errno;
use ksync::cell::SyncUnsafeCell;
use memory::{
    address::{PhysAddr, PhysPageNum, StepOne},
    frame::{frame_alloc, frame_dealloc, FrameTracker},
};
use virtio_drivers_async::{BufferDirection, Hal, PhysAddr as VirtioPhysAddr};

lazy_static::lazy_static! {
    pub static ref QUEUE_FRAMES: SyncUnsafeCell<Vec<FrameTracker>> = SyncUnsafeCell::new(Vec::new());
}

pub struct VirtioHalImpl;

unsafe impl Hal for VirtioHalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (VirtioPhysAddr, NonNull<u8>) {
        let mut ppn_base = PhysPageNum::from(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn();
            }
            assert_eq!(frame.ppn().raw(), ppn_base.raw() + i);
            QUEUE_FRAMES.as_ref_mut().push(frame);
        }
        let paddr = PhysAddr::from(PhysPageNum::from(ppn_base));
        let vaddr = NonNull::new((paddr.raw() | KERNEL_ADDR_OFFSET) as *mut u8).unwrap();
        (paddr.raw(), vaddr)
    }
    unsafe fn dma_dealloc(paddr: VirtioPhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let pa = PhysAddr::from(paddr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }
    #[inline]
    unsafe fn mmio_phys_to_virt(paddr: VirtioPhysAddr, _size: usize) -> NonNull<u8> {
        let vaddr = paddr | KERNEL_ADDR_OFFSET;
        NonNull::new(vaddr as _).unwrap()
    }
    // #[no_mangle]
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> VirtioPhysAddr {
        // Nothing to do, as the host already has access to all memory.
        let phys = buffer.as_ptr() as *mut u8 as usize - KERNEL_ADDR_OFFSET;
        VirtioPhysAddr::from(phys)
    }
    #[inline]
    unsafe fn unshare(_paddr: VirtioPhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do
    }
}

#[allow(dead_code)]
pub const fn dev_err(err: virtio_drivers_async::Error) -> Errno {
    use virtio_drivers_async::Error::*;
    match err {
        QueueFull => Errno::EAGAIN,
        NotReady => Errno::EAGAIN,
        WrongToken => Errno::EADDRINUSE, // this
        InvalidParam => Errno::EINVAL,
        IoError => Errno::EIO,
        Unsupported => Errno::ENOSYS,
        ConfigSpaceTooSmall => Errno::EINVAL,
        ConfigSpaceMissing => Errno::EINVAL,
        _ => Errno::EINVAL,
    }
}

use core::{f32::consts::E, ptr::NonNull};

use arch::ArchMemory;
use include::errno::Errno;
use virtio_drivers::{BufferDirection, Hal, PhysAddr};

pub struct VirtioHalImpl;

unsafe impl Hal for VirtioHalImpl {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        /*
        fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
            let vaddr = if let Ok(vaddr) = global_allocator().alloc_pages(pages, 0x1000) {
                vaddr
            } else {
                return (0, NonNull::dangling());
            };
            let paddr = virt_to_phys(vaddr.into());
            let ptr = NonNull::new(vaddr as _).unwrap();
            (paddr.as_usize(), ptr)
        }
        */
        todo!(" ↑ do like this ↑");
    }
    unsafe fn dma_dealloc(paddr: PhysAddr, vaddr: NonNull<u8>, pages: usize) -> i32 {
        /*
        unsafe fn dma_dealloc(_paddr: PhysAddr, vaddr: NonNull<u8>, pages: usize) -> i32 {
            global_allocator().dealloc_pages(vaddr.as_ptr() as usize, pages);
            0
        }
        */
        todo!(" ↑ do like this ↑");
    }
    #[inline]
    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, _size: usize) -> NonNull<u8> {
        let vaddr = paddr | arch::Arch::KERNEL_ADDR_OFFSET;
        NonNull::new(vaddr as _).unwrap()
    }
    #[inline]
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        vaddr - arch::Arch::KERNEL_ADDR_OFFSET
    }
    #[inline]
    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {}
}

#[allow(dead_code)]
pub const fn dev_err(err: virtio_drivers::Error) -> Errno {
    use virtio_drivers::Error::*;
    match err {
        QueueFull => Errno::EAGAIN,
        NotReady => Errno::EAGAIN,
        WrongToken => Errno::EINVAL,
        InvalidParam => Errno::EINVAL,
        IoError => Errno::EIO,
        Unsupported => Errno::ENOSYS,
        ConfigSpaceTooSmall => Errno::EINVAL,
        ConfigSpaceMissing => Errno::EINVAL,
        _ => Errno::EINVAL,
    }
}

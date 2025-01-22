//! The examples provided by virtio_drivers serve as references for
//! implementation. [reference](https://github.com/rcore-os/virtio-drivers/tree/master/examples/riscv)

use alloc::vec::Vec;
use core::ptr::NonNull;

use ksync::mutex::SpinLock;
use lazy_static::lazy_static;
// use spin::Mutex;
use virtio_drivers::{BufferDirection, Hal, PhysAddr as VirtioPhysAddr};

type Mutex<T> = ksync::mutex::SpinLock<T>;
// type MutexGuard<'a, T> = ksync::mutex::SpinLockGuard<'a, T>;

use crate::{
    config::mm::KERNEL_ADDR_OFFSET,
    mm::{
        address::{PhysAddr, PhysPageNum, StepOne, VirtAddr},
        frame::{frame_alloc, frame_dealloc, FrameTracker},
        memory_set::KERNEL_SPACE,
        page_table::PageTable,
    },
    utils::{kernel_pa_to_va, kernel_va_to_pa},
};

static DMA_PADDR: SpinLock<Vec<FrameTracker>> = SpinLock::new(Vec::new());

pub struct HalImpl;

// ? modify is not sure
unsafe impl Hal for HalImpl {
    // 需要加回去
    //#[no_mangle]
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (VirtioPhysAddr, NonNull<u8>) {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc();
            if i == 0 {
                ppn_base = frame.ppn();
            }
            assert_eq!(frame.ppn().0, ppn_base.0 + i);
            DMA_PADDR.lock().push(frame);
        }
        // let kpaddr: KPhysAddr = ppn_base.into();
        let paddr = PhysAddr::from(PhysPageNum::from(ppn_base));
        let vaddr = NonNull::new((paddr.0 | KERNEL_ADDR_OFFSET) as *mut u8).unwrap();
        (paddr.0, vaddr)
    }
    // #[no_mangle]
    unsafe fn dma_dealloc(paddr: VirtioPhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        // let pa: KPhysAddr = paddr.into();
        let pa = PhysAddr::from(paddr);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }

        0
    }
    //#[no_mangle]
    unsafe fn mmio_phys_to_virt(paddr: VirtioPhysAddr, _size: usize) -> NonNull<u8> {
        // let vaddr = KPhysAddr::from(paddr).0 + KERNEL_ADDR_OFFSET;
        let vaddr = paddr | KERNEL_ADDR_OFFSET;
        NonNull::new(vaddr as _).unwrap()
    }
    // #[no_mangle]
    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> VirtioPhysAddr {
        // Nothing to do, as the host already has access to all memory.
        let phys = buffer.as_ptr() as *mut u8 as usize - KERNEL_ADDR_OFFSET;
        // KPhysAddr::from(phys).0
        VirtioPhysAddr::from(phys)
    }
    //#[no_mangle]
    unsafe fn unshare(_paddr: VirtioPhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // Nothing to do
    }
}

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
}

#[no_mangle]
// pub extern "C" fn virtio_dma_alloc(pages: usize) -> KPhysAddr {
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    println!("virtio_dma_alloc: {}", pages);
    let mut ppn_base = 0;
    for i in 0..pages {
        let frame = frame_alloc();
        if i == 0 {
            ppn_base = frame.ppn().into();
        }
        let frame_ppn: usize = frame.ppn().into();
        assert_eq!(frame_ppn, ppn_base + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    PhysAddr::from(PhysPageNum::from(ppn_base))
}

#[no_mangle]
// pub extern "C" fn virtio_dma_dealloc(pa: KPhysAddr, pages: usize) -> i32 {
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    println!("virtio_dma_dealloc: {:#x}, {}", pa.0, pages);
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
// pub extern "C" fn virtio_phys_to_virt(paddr: KPhysAddr) -> VirtAddr {
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    println!("virtio_phys_to_virt: {:#x}", paddr.0);
    VirtAddr::from(kernel_pa_to_va(paddr.0))
}

#[no_mangle]
// pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> KPhysAddr {
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    println!("virtio_virt_to_phys: {:#x}", vaddr.0);
    // PageTable::from_token(KERNEL_SPACE.lock().token())
    //     .translate_va(vaddr)
    //     .unwrap()
    //     .into()
    // ! fixme
    // ! fix additonal translation
    let translate_pa: PhysAddr = PageTable::from_token(KERNEL_SPACE.lock().token())
        .translate_va(vaddr)
        .unwrap()
        .into();
    let pa = PhysAddr::from(kernel_va_to_pa(vaddr.0));
    assert_eq!(pa, translate_pa, "virtio_virt_to_phys translation failed");
    pa
}

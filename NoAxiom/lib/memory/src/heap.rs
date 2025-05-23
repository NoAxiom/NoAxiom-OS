//! The global heap allocator

use core::alloc::GlobalAlloc;

use buddy_system_allocator::LockedHeap;
use config::mm::KERNEL_HEAP_SIZE;
use ksync::mutex::{LockAction, NoIrqLockAction};

struct NoIrqHeapAllocator(LockedHeap<32>);

unsafe impl GlobalAlloc for NoIrqHeapAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        NoIrqLockAction::before_lock();
        let res = self.0.alloc(layout);
        NoIrqLockAction::after_lock();
        res
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        NoIrqLockAction::before_lock();
        self.0.dealloc(ptr, layout);
        NoIrqLockAction::after_lock();
    }
}

#[global_allocator]
static HEAP_ALLOCATOR: NoIrqHeapAllocator = NoIrqHeapAllocator(LockedHeap::empty());

#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    if let Some(heap) = HEAP_ALLOCATOR.0.try_lock() {
        error!("{:?}", heap);
    } else {
        error!("HEAP_ALLOCATOR is already locked");
    }
    panic!("Heap allocation error, layout = {:?}", layout);
}

/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// initiate heap allocator
pub fn heap_init() {
    unsafe {
        HEAP_ALLOCATOR
            .0
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::{boxed::Box, vec::Vec};
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    debug_assert_eq!(*a, 5);
    debug_assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for (i, val) in v.iter().take(500).enumerate() {
        debug_assert_eq!(*val, i);
    }
    debug_assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    info!("heap_test passed!");
}

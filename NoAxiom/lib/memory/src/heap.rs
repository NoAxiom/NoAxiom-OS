//! The global heap allocator

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::NonNull,
};

use buddy_system_allocator::Heap;
use config::mm::KERNEL_HEAP_SIZE;
use console::println;
use ksync::mutex::SpinLock;

#[global_allocator]
static HEAP_ALLOCATOR: HeapAllocator = HeapAllocator::empty();

struct HeapAllocator(SpinLock<Heap<32>>);

impl HeapAllocator {
    const fn empty() -> Self {
        Self(SpinLock::new(Heap::empty()))
    }
}
unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .alloc(layout)
            .ok()
            .map_or(0 as *mut u8, |allocation| allocation.as_ptr())
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub fn print_heap_info() {
    if let Some(heap) = HEAP_ALLOCATOR.0.try_lock() {
        let user = heap.stats_alloc_user();
        let actual = heap.stats_alloc_actual();
        let total = heap.stats_total_bytes();
        // calc in usize
        println!("[heap] {:?}", heap);
        println!(
            "[heap] alloc: {}%, real-used: {}%, utilization: {}%",
            actual * 100 / total,
            user * 100 / total,
            user * 100 / actual,
        );
    } else {
        println!("[heap] HEAP_ALLOCATOR is already locked");
    }
}

#[alloc_error_handler]
/// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    print_heap_info();
    panic!("Heap allocation error, layout = {:?}", layout);
}

/// heap space ([u8; KERNEL_HEAP_SIZE])
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// initiate heap allocator
/// dont call println since the console isn't fully initialized yet
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

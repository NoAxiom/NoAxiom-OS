use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::fmt::{self, Debug, Formatter};

use arch::consts::KERNEL_PHYS_MEMORY_END;
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use super::address::{PhysPageNum, VirtPageNum};
use crate::{
    address::PhysAddr,
    utils::{kernel_ppn_to_vpn, kernel_va_to_pa},
};

/// frame tracker inner
pub struct FrameTrackerInner {
    ppn: PhysPageNum,
}
impl FrameTrackerInner {
    #[inline]
    fn new_uninit(ppn: PhysPageNum) -> Self {
        Self { ppn }
    }
}
impl Debug for FrameTrackerInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTrackerInner:PPN={:#x}", self.ppn.0))
    }
}
impl Drop for FrameTrackerInner {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

/// frame tracker, with ref count
#[derive(Debug)]
pub struct FrameTracker {
    inner: Arc<FrameTrackerInner>,
}
impl FrameTracker {
    fn new(inner: FrameTrackerInner) -> Self {
        let inner = Arc::new(inner);
        Self { inner }
    }
    #[inline]
    pub fn fill_zero(&self) {
        self.inner.ppn.get_bytes_array().fill(0);
    }
    #[inline]
    pub fn fill_data(&self, src: &[u8]) {
        self.inner.ppn.get_bytes_array().copy_from_slice(src);
    }
    #[inline(always)]
    pub fn ppn(&self) -> PhysPageNum {
        self.inner.ppn
    }
    #[inline(always)]
    pub fn kernel_vpn(&self) -> VirtPageNum {
        VirtPageNum::from(kernel_ppn_to_vpn(self.inner.ppn.0))
    }
}
impl Clone for FrameTracker {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct FrameTrackerRaw(FrameTracker);
impl FrameTrackerRaw {
    pub fn new(ppn: PhysPageNum) -> Self {
        Self(FrameTracker::new(FrameTrackerInner::new_uninit(ppn)))
    }
    pub fn zero_inited(self) -> FrameTracker {
        self.0.fill_zero();
        self.0
    }
    pub unsafe fn keep_uninited(self) -> FrameTracker {
        self.0
    }
    pub fn data_inited(self, src: &[u8]) -> FrameTracker {
        self.0.fill_data(src);
        self.0
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
    frame_map: BTreeMap<usize, Weak<FrameTrackerInner>>,
}
impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        info!(
            "last {} Physical Frames. start: {:#x}, end: {:#x}",
            (self.end - self.current) as isize,
            self.current,
            self.end
        );
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
            frame_map: BTreeMap::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            // trace!("[frame] recycled use: frame ppn={:#x}", self.current);
            Some(ppn.into())
        } else if self.current == self.end {
            error!("[frame] out of memory!");
            None
        } else {
            // trace!("[frame] alloc frame ppn={:#x}", self.current);
            self.current += 1;
            Some((self.current - 1).into())
        }
    }
    /// dealloc frame
    /// SAFETY: check if the satp is correctly switched to other before dealloc
    /// NOTE THAT the deallocation won't clear the data in
    /// corrisponding frame, so the processor can run on a deallocated
    /// page, which can possibly cause pagefault after the page being
    /// allocated again.
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // validity check
        // FIXME: only for debug, remove it in release
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: SpinLock<FrameAllocatorImpl> =
        SpinLock::new(FrameAllocatorImpl::new());
}

pub fn frame_alloc() -> FrameTracker {
    let mut guard = FRAME_ALLOCATOR.lock();
    let ppn = guard.alloc().unwrap();
    let frame = FrameTrackerRaw::new(ppn).zero_inited();
    guard.frame_map.insert(ppn.0, Arc::downgrade(&frame.inner));
    frame
}

#[allow(unused)]
pub fn frame_alloc_raw() -> FrameTrackerRaw {
    let mut guard = FRAME_ALLOCATOR.lock();
    let ppn = guard.alloc().unwrap();
    let frame = FrameTrackerRaw::new(ppn);
    guard
        .frame_map
        .insert(ppn.0, Arc::downgrade(&frame.0.inner));
    frame
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    let mut guard = FRAME_ALLOCATOR.lock();
    guard.dealloc(ppn);
    // FIXME: only for debug, remove it in release
    assert!(guard.frame_map.contains_key(&ppn.0));
    guard.frame_map.remove(&ppn.0);
}

pub fn frame_refcount(ppn: PhysPageNum) -> usize {
    FRAME_ALLOCATOR
        .lock()
        .frame_map
        .get(&ppn.0)
        .map_or(0, |x| x.strong_count())
}

/// init frame allocator
pub fn frame_init() {
    extern "C" {
        fn ekernel(); // virt address
    }
    FRAME_ALLOCATOR.lock().init(
        PhysAddr::from(kernel_va_to_pa(ekernel as usize)).ceil(),
        PhysAddr::from(KERNEL_PHYS_MEMORY_END).floor(),
    );
    info!("[frame_init] frame allocator init success.");
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc();
        debug!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc();
        debug!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    debug!("frame_allocator_test passed!");
}

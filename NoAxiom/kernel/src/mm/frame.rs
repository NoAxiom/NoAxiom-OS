use alloc::{boxed::Box, collections::{btree_map::BTreeMap, vec_deque::VecDeque}, string::String, vec::Vec};
use core::fmt::{self, Debug, Formatter};

use lazy_static::lazy_static;

use super::address::PhysPageNum;
use crate::{
    config::mm::KERNEL_PHYS_MEMORY_END, mm::address::PhysAddr, println, sync::mutex::SpinMutex,
    utils::kernel_va_to_pa,
};

pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }
        // debug!("FrameTracker: new ppn={:#x}", ppn.0);
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
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
    recycled: VecDeque<usize>,
    // mapped: BTreeMap<usize, bool>, // ppn -> is_mapped
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
        println!("last {} Physical Frames.", self.end - self.current);
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: VecDeque::new(),
            // mapped: BTreeMap::new(),
        }
    }
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if !self.recycled.is_empty() {
            trace!(
                "before: {:?}",
                self.recycled
                    .iter()
                    .map(|x| format!("{:#x}", x))
                    .collect::<Vec<String>>()
            );
        }
        let res: Option<PhysPageNum> = if let Some(ppn) = self.recycled.pop_front() {
            debug!(
                "[frame] recycled use: frame ppn = {:#x}, current_total_allocation: {:#x}",
                ppn, self.current
            );
            trace!(
                "after: {:?}",
                self.recycled
                    .iter()
                    .map(|x| format!("{:#x}", x))
                    .collect::<Vec<String>>()
            );
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            trace!("[frame] alloc frame ppn={:#x}", self.current);
            self.current += 1;
            Some((self.current - 1).into())
        };
        // let value = res.unwrap().0;
        // assert!(self.mapped.get(&value).is_none());
        // self.mapped.insert(value, true);
        res
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        debug!("dealloc: {:#x}", ppn);
        // let data = Box::new(0);
        // warn!("qwqwq: {}", data);
        // assert!(self.mapped.get(&ppn).is_some());
        // self.mapped.remove(&ppn);
        // debug!("qwq");
        // validity check
        if ppn >= self.current || self.recycled.iter().any(|&v| v == ppn) {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push_back(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: SpinMutex<FrameAllocatorImpl> =
        SpinMutex::new(FrameAllocatorImpl::new());
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc().map(|x| FrameTracker::new(x))
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    trace!("frame_dealloc in");
    FRAME_ALLOCATOR.lock().dealloc(ppn);
    trace!("frame_dealloc done");
}

/// init frame allocator
pub fn frame_init() {
    extern "C" {
        fn ekernel(); // virt address
    }
    let mut guard = FRAME_ALLOCATOR.lock();
    guard.init(
        PhysAddr::from(kernel_va_to_pa(ekernel as usize)).ceil(),
        PhysAddr::from(KERNEL_PHYS_MEMORY_END).floor(),
    );
    info!("FRAME_ALLOCATOR: {:#x}, {:#x}", guard.current, guard.end);
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        trace!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        trace!("{:?}", frame);
        v.push(frame);
    }
    drop(v);
    trace!("frame_allocator_test passed!");
}

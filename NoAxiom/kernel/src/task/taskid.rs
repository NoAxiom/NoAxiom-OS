use alloc::vec::Vec;

use crate::sync::mutex::SpinMutex;

/// Task ID allocator
struct IndexAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl IndexAllocator {
    const fn new() -> Self {
        IndexAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> usize {
        if let Some(tid) = self.recycled.pop() {
            tid
        } else {
            self.current += 1;
            self.current
        }
    }

    fn dealloc(&mut self, tid: usize) {
        debug_assert!(tid <= self.current);
        debug_assert!(
            !self.recycled.iter().any(|ttid| *ttid == tid),
            "tid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

static TID_ALLOCATOR: SpinMutex<IndexAllocator> = SpinMutex::new(IndexAllocator::new());
// static PID_ALLOCATOR: SpinMutex<IndexAllocator> =
// SpinMutex::new(IndexAllocator::new());

/// task id with auto dealloc
pub struct TidTracer(pub usize);
impl Into<usize> for TidTracer {
    fn into(self) -> usize {
        self.0
    }
}
impl From<usize> for TidTracer {
    fn from(tid: usize) -> Self {
        TidTracer(tid)
    }
}
impl Drop for TidTracer {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}
pub fn tid_alloc() -> TidTracer {
    TID_ALLOCATOR.lock().alloc().into()
}

// /// pid with auto dealloc
// pub struct PidTracer(pub usize);
// impl Into<usize> for PidTracer {
//     fn into(self) -> usize {
//         self.0
//     }
// }
// impl From<usize> for PidTracer {
//     fn from(pid: usize) -> Self {
//         PidTracer(pid)
//     }
// }
// impl Drop for PidTracer {
//     fn drop(&mut self) {
//         PID_ALLOCATOR.lock().dealloc(self.0);
//     }
// }
// pub fn pid_alloc() -> PidTracer {
//     PID_ALLOCATOR.lock().alloc().into()
// }

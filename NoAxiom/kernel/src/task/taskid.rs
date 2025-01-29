use alloc::vec::Vec;

use ksync::mutex::SpinLock;

pub type TID = usize;
pub type TGID = TID;
pub type PID = TGID;
pub type PGID = usize;

/// Task ID allocator
struct IndexAllocator {
    current: TID,
    recycled: Vec<TID>,
}

impl IndexAllocator {
    const fn new() -> Self {
        IndexAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> TID {
        if let Some(tid) = self.recycled.pop() {
            tid
        } else {
            self.current += 1;
            self.current
        }
    }

    fn dealloc(&mut self, tid: TID) {
        debug_assert!(tid <= self.current);
        debug_assert!(
            !self.recycled.iter().any(|ttid| *ttid == tid),
            "tid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

static TID_ALLOCATOR: SpinLock<IndexAllocator> = SpinLock::new(IndexAllocator::new());

/// task id with auto dealloc
pub struct TidTracer(pub TID);
impl Into<TID> for TidTracer {
    fn into(self) -> TID {
        self.0
    }
}
impl From<TID> for TidTracer {
    fn from(tid: TID) -> Self {
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

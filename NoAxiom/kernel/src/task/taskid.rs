use alloc::vec::Vec;

use crate::sync::mutex::SpinMutex;

/// Task ID allocator
struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    const fn new() -> Self {
        TidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> TaskId {
        if let Some(tid) = self.recycled.pop() {
            TaskId(tid)
        } else {
            self.current += 1;
            TaskId(self.current)
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

static TID_ALLOCATOR: SpinMutex<TidAllocator> = SpinMutex::new(TidAllocator::new());

/// 使用TaskID包装, 保证自动Dealloc
pub struct TaskId(pub usize);

impl Into<usize> for TaskId {
    fn into(self) -> usize {
        self.0
    }
}

impl From<usize> for TaskId {
    fn from(tid: usize) -> Self {
        TaskId(tid)
    }
}

impl Drop for TaskId {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub fn tid_alloc() -> TaskId {
    TID_ALLOCATOR.lock().alloc()
}

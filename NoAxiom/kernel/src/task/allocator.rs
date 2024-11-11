use alloc::vec::Vec;

use ksync::mutex::SpinMutex;

struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    pub const fn new() -> Self {
        TidAllocator {
            current: 1,
            recycled: vec![],
        }
    }

    pub fn alloc(&mut self) -> TaskId {
        if let Some(tid) = self.recycled.pop() {
            TaskId(tid)
        } else {
            self.current += 1;
            TaskId(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, tid: usize) {
        assert!(tid < self.current);
        assert!(
            !self.recycled.iter().any(|ttid| *ttid == tid),
            "tid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

static TID_ALLOCATOR: SpinMutex<TidAllocator> = SpinMutex::new(TidAllocator::new());

/// bind task id lifetime
pub struct TaskId(pub usize);

impl Drop for TaskId {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}

pub fn tid_alloc() -> TaskId {
    TID_ALLOCATOR.lock().alloc()
}

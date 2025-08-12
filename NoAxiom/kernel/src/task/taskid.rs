use alloc::vec::Vec;

use ksync::mutex::SpinLock;

pub type TID = usize;
pub type TGID = TID;
pub type PID = TGID;
pub type PGID = TID;

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

#[derive(Debug, Clone)]
pub struct TaskUserId {
    /// user id
    pub uid: u32,
    /// group id
    pub gid: u32,
    /// user id - file system
    pub fsuid: u32,
    /// group id - file system
    pub fsgid: u32,
    /// user id - effective
    pub euid: u32,
    /// group id - effective
    pub egid: u32,
    /// user id - saved
    pub suid: u32,
    /// group id - saved
    pub sgid: u32,
}

impl TaskUserId {
    #[inline(always)]
    pub fn uid(&self) -> u32 {
        self.uid
    }
    #[inline(always)]
    pub fn gid(&self) -> u32 {
        self.gid
    }
    #[inline(always)]
    pub fn fsuid(&self) -> u32 {
        self.fsuid
    }
    #[inline(always)]
    pub fn fsgid(&self) -> u32 {
        self.fsgid
    }
    #[inline(always)]
    pub fn set_uid(&mut self, uid: u32) {
        self.uid = uid
    }
    #[inline(always)]
    pub fn set_gid(&mut self, gid: u32) {
        self.gid = gid
    }
    #[inline(always)]
    pub fn set_fsuid(&mut self, fsuid: u32) {
        self.fsuid = fsuid
    }
    #[inline(always)]
    pub fn set_fsgid(&mut self, fsgid: u32) {
        self.fsgid = fsgid
    }
    #[inline(always)]
    pub fn euid(&self) -> u32 {
        self.euid
    }
    #[inline(always)]
    pub fn egid(&self) -> u32 {
        self.egid
    }
    #[inline(always)]
    pub fn set_euid(&mut self, euid: u32) {
        self.euid = euid
    }
    #[inline(always)]
    pub fn set_egid(&mut self, egid: u32) {
        self.egid = egid
    }
    #[inline(always)]
    pub fn suid(&self) -> u32 {
        self.suid
    }
    #[inline(always)]
    pub fn set_suid(&mut self, suid: u32) {
        self.suid = suid
    }
    #[inline(always)]
    pub fn sgid(&self) -> u32 {
        self.sgid
    }
    #[inline(always)]
    pub fn set_sgid(&mut self, sgid: u32) {
        self.sgid = sgid;
    }
}

impl Default for TaskUserId {
    fn default() -> Self {
        TaskUserId {
            uid: 0,
            gid: 0,
            fsuid: 0,
            fsgid: 0,
            euid: 0,
            egid: 0,
            suid: 0,
            sgid: 0,
        }
    }
}

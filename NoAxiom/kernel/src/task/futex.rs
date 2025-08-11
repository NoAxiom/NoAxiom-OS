use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    sync::Arc,
};
use core::{
    fmt::Debug,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, Waker},
};

use include::errno::{Errno, SysResult};
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};
use lazy_static::lazy_static;
use memory::address::{PhysAddr, VirtAddr};

use crate::{
    cpu::current_task,
    include::futex::{FutexFlags, FUTEX_BITSET_MATCH_ANY},
    mm::user_ptr::UserPtr,
    syscall::SyscallResult,
};

pub struct FutexWaiter {
    waker: Waker,
    bitset: u32,
    done: Arc<AtomicBool>,
}

impl FutexWaiter {
    pub fn new(waker: Waker, bitset: u32, done: Arc<AtomicBool>) -> Self {
        Self {
            waker,
            bitset,
            done,
        }
    }
}

type WaiterQueueInner = VecDeque<FutexWaiter>;
pub struct WaiterQueue(WaiterQueueInner);
impl WaiterQueue {
    pub fn new() -> Self {
        Self(VecDeque::new())
    }
}
impl Deref for WaiterQueue {
    type Target = WaiterQueueInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for WaiterQueue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FutexAddr {
    Private(VirtAddr),
    Shared(PhysAddr),
}

impl FutexAddr {
    pub async fn new(uaddr: usize, flags: FutexFlags) -> SysResult<Self> {
        if flags.is_private() {
            Ok(FutexAddr::Private(VirtAddr::from(uaddr)))
        } else {
            let futex_word = UserPtr::<u32>::new(uaddr);
            let pa = futex_word.translate_pa().await?;
            Ok(FutexAddr::Shared(pa))
        }
    }
    pub fn new_private(uaddr: usize) -> VirtAddr {
        VirtAddr::from(uaddr)
    }
    pub async fn new_shared(uaddr: usize) -> SysResult<PhysAddr> {
        let futex_word = UserPtr::<u32>::new(uaddr);
        let pa = futex_word.translate_pa().await?;
        Ok(pa)
    }
}

/// futex queue: a map of uaddr -> waiter queue
pub struct FutexQueue<T> {
    inner: BTreeMap<T, WaiterQueue>,
}

pub type FutexPrivateQueue = FutexQueue<VirtAddr>;

lazy_static! {
    pub static ref FUTEX_SHARED_QUEUE: SpinLock<FutexQueue<PhysAddr>> =
        SpinLock::new(FutexQueue::new());
}

impl<T> FutexQueue<T>
where
    T: Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd,
{
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// insert a new waiter into the queue
    pub fn insert_waiter(&mut self, addr: T, waker: Waker, bitset: u32, done: Arc<AtomicBool>) {
        debug!(
            "[futex] insert waiter at addr: {:?}, bitset: {:x}",
            addr, bitset
        );
        let waiter = FutexWaiter::new(waker, bitset, done);
        if let Some(waiters) = self.inner.get_mut(&addr) {
            debug!(
                "[futex] found existing waiters at addr: {:?}, adding new waker",
                addr
            );
            waiters.push_back(waiter);
        } else {
            debug!(
                "[futex] no waiters found at addr: {:?}, creating new queue",
                addr
            );
            let mut waiters = WaiterQueue::new();
            waiters.push_back(waiter);
            self.inner.insert(addr, waiters);
        }
    }

    pub fn get_waiter_queue(&mut self, addr: T) -> Option<&mut WaiterQueue> {
        self.inner.get_mut(&addr)
    }

    /// wake up all valid waiters, return the number of waiters woken up
    pub fn wake_waiter(&mut self, addr: T, wake_num: u32, bitset: u32) -> usize {
        let mut count = 0;
        debug!(
            "[futex] wake waiters at addr: {:x?}, bitset: {:x}, wake_num: {}",
            addr, bitset, wake_num
        );
        for it in self.inner.iter() {
            debug!("[futex] addr: {:x?}, waiters: {}", it.0, it.1.len());
        }
        if let Some(waiters) = self.get_waiter_queue(addr) {
            debug!("[futex] found {} waiters", waiters.len());
            debug!("[futex] wake_num: {}, bitset: {:x}", wake_num, bitset);
            let mut tmp_waiters = VecDeque::new();
            while let Some(waiter) = waiters.pop_front() {
                if waiter.bitset & bitset == 0 {
                    tmp_waiters.push_back(waiter);
                } else {
                    if Arc::strong_count(&waiter.done) > 1 {
                        waiter.done.store(true, Ordering::SeqCst);
                        waiter.waker.wake();
                    } else {
                        warn!("[futex] waker already dropped, wake canceled");
                    }
                    count += 1;
                    if count >= wake_num {
                        break;
                    }
                }
            }
            waiters.append(&mut tmp_waiters);
        }
        count as usize
    }

    /// requeue the waiters from old_pa to new_pa
    /// n_wake: the max number of waiters to wake up
    /// n_rq: the max number of waiters to requeue
    /// return the sum number of waiters woken up and requeued
    pub fn requeue(&mut self, old_addr: T, new_addr: T, n_wake: u32, n_rq: u32) -> usize {
        // first wake up the waiters in old_pa
        let wake_count = self.wake_waiter(old_addr, n_wake, FUTEX_BITSET_MATCH_ANY);
        let Some(old_waiters) = self.get_waiter_queue(old_addr) else {
            return wake_count;
        };

        // now try to requeue the waiters from old_pa to new_pa
        // waiter_vec is a temporary vector to store the waiters
        if old_addr == new_addr {
            return 0;
        }
        let mut rq_count = 0;
        let mut waiter_vec = VecDeque::new();

        // pop the waiters from old_pa and push them to new_pa
        while let Some(waker) = old_waiters.pop_front() {
            waiter_vec.push_back(waker);
            rq_count += 1;
            if rq_count == n_rq {
                break;
            }
        }

        // now we have the waiters in waiter_vec, we need to insert them into new_pa
        self.inner
            .entry(new_addr)
            .or_insert(WaiterQueue::new())
            .append(&mut waiter_vec);

        rq_count as usize + wake_count
    }
}

pub struct FutexFuture<'a, T>
where
    T: Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd,
{
    uaddr: usize, // va
    faddr: T,
    guard: &'a SpinLock<FutexQueue<T>>,
    val: u32,
    bitset: u32, // for bitset futex
    is_in: SyncUnsafeCell<bool>,
    done: Arc<AtomicBool>,
}

impl<'a, T> FutexFuture<'a, T>
where
    T: Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd,
{
    pub fn new(
        uaddr: usize,
        faddr: T,
        val: u32,
        bitset: u32,
        guard: &'a SpinLock<FutexQueue<T>>,
    ) -> Self {
        Self {
            uaddr,
            faddr,
            guard,
            val,
            bitset,
            is_in: SyncUnsafeCell::new(false),
            done: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<'a, T> Future for FutexFuture<'a, T>
where
    T: Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd,
{
    type Output = SyscallResult;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        debug!(
            "[futex] poll uaddr: {:x}, faddr: {:?}, val: {}, bitset: {:x}",
            self.uaddr, self.faddr, self.val, self.bitset
        );
        if !*self.is_in.as_ref() {
            let mut futex = self.guard.lock();
            let cur_val = unsafe { UserPtr::from(self.uaddr as *const u32).atomic_load_acquire() };
            let is_pending = cur_val == self.val;
            if is_pending {
                *self.is_in.as_ref_mut() = true;
                futex.insert_waiter(
                    self.faddr,
                    cx.waker().clone(),
                    self.bitset,
                    self.done.clone(),
                );
                debug!(
                    "[futex] task {} yield with value = {}",
                    current_task().unwrap().tid(),
                    self.val
                );
                return Poll::Pending;
            } else {
                // failed to lock, remove the waker
                return Poll::Ready(Err(Errno::EAGAIN));
            };
        } else {
            match self.done.load(Ordering::SeqCst) {
                true => Poll::Ready(Ok(0)),
                false => Poll::Pending,
            }
        }
    }
}

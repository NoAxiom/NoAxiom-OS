use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    vec::Vec,
};
use core::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use include::errno::Errno;
use ksync::cell::SyncUnsafeCell;
use memory::address::PhysAddr;

use super::taskid::TID;
use crate::{cpu::current_task, mm::user_ptr::UserPtr, syscall::SyscallResult};

/// waiter queue: a map of TID -> Waker
type WaiterQueueInner = VecDeque<Waker>;
pub struct WaiterQueue(SyncUnsafeCell<WaiterQueueInner>);
impl WaiterQueue {
    pub fn new() -> Self {
        Self(SyncUnsafeCell::new(VecDeque::new()))
    }
}
impl Deref for WaiterQueue {
    type Target = WaiterQueueInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl DerefMut for WaiterQueue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_mut()
    }
}

/// futex queue: a map of uaddr -> waiter queue
pub struct FutexQueue {
    inner: BTreeMap<PhysAddr, WaiterQueue>,
}

impl FutexQueue {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// insert a new waiter into the queue
    pub fn insert_waiter(&mut self, pa: PhysAddr, tid: TID, waker: Waker) {
        if let Some(waiters) = self.inner.get_mut(&pa) {
            waiters.insert(tid, waker);
        } else {
            let mut waiters = WaiterQueue::new();
            waiters.insert(tid, waker);
            self.inner.insert(pa, waiters);
        }
    }

    /// get the waiter queue from a given uaddr
    pub fn get_waiter_queue(&mut self, pa: PhysAddr) -> Option<&mut WaiterQueue> {
        self.inner.get_mut(&pa)
    }

    /// wake up all valid waiters, return the number of waiters woken up
    pub fn wake_waiter(&mut self, pa: PhysAddr, wake_num: u32) -> usize {
        let mut count = 0;
        if let Some(waiters) = self.get_waiter_queue(pa) {
            while let Some(waker) = waiters.pop_front() {
                waker.wake();
                count += 1;
                if count >= wake_num {
                    break;
                }
            }
        }
        count as usize
    }

    /// requeue the waiters from old_pa to new_pa
    /// n_wake: the max number of waiters to wake up
    /// n_rq: the max number of waiters to requeue
    /// return the sum number of waiters woken up and requeued
    pub fn requeue(&mut self, old_pa: PhysAddr, new_pa: PhysAddr, n_wake: u32, n_rq: u32) -> usize {
        // first wake up the waiters in old_pa
        let wake_count = self.wake_waiter(old_pa, n_wake);
        let Some(old_waiters) = self.get_waiter_queue(old_pa) else {
            return wake_count;
        };

        // now try to requeue the waiters from old_pa to new_pa
        // waiter_vec is a temporary vector to store the waiters
        if old_pa == new_pa {
            return 0;
        }
        let mut rq_count = 0;
        let mut waiter_vec = Vec::new();

        // pop the waiters from old_pa and push them to new_pa
        while let Some(waker) = old_waiters.pop_front() {
            waiter_vec.push(waker);
            rq_count += 1;
            if rq_count == n_rq {
                break;
            }
        }

        // now we have the waiters in waiter_vec, we need to insert them into new_pa
        self.inner
            .entry(new_pa)
            .or_insert_with(WaiterQueue::new)
            .extend(waiter_vec);

        rq_count as usize + wake_count
    }
}

pub struct FutexFuture {
    uaddr: usize, // va
    pa: PhysAddr,
    val: u32,
    is_in: bool,
}

impl FutexFuture {
    pub fn new(uaddr: usize, pa: PhysAddr, val: u32) -> Self {
        Self {
            uaddr,
            pa,
            val,
            is_in: false,
        }
    }
}

impl Future for FutexFuture {
    type Output = SyscallResult;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let task = current_task();
        let mut futex = task.futex();
        if !self.is_in {
            if UserPtr::from(self.uaddr as *const u32).atomic_load_acquire() == self.val {
                self.is_in = true;
                futex.insert_waiter(self.pa, task.tid(), cx.waker().clone());
                return Poll::Pending;
            } else {
                // failed to lock, remove the waker
                return Poll::Ready(Err(Errno::EAGAIN));
            };
        }
        Poll::Ready(Ok(0))
    }
}

/*

Futex是一个主要运行与用户空间的互斥锁, 用于竞争较少的情况
主要维护思想是: 先尝试在用户空间当中进行上锁, 如果上锁成功则直接使用
如果上锁失败, 再陷入内核当中插入到waitqueue当中进行等待
这样可以在竞争较小的情况下尽量的避免内核陷入
内核当中的sys_futex是用于维护上锁失败时的等待的

*/

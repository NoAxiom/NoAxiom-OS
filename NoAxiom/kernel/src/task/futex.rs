use alloc::collections::btree_map::BTreeMap;
use core::{
    future::Future,
    task::{Poll, Waker},
};

use super::taskid::TID;
use crate::mm::user_ptr::UserPtr;

pub struct FutexQueue {
    queue: BTreeMap<u32, BTreeMap<TID, Waker>>,
}

impl FutexQueue {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
        }
    }
    pub fn wake(&mut self, uaddr: UserPtr<u32>, val: u32) -> usize {
        todo!()
    }
    pub fn requeue_waiters(
        &mut self,
        uaddr: UserPtr<u32>,
        uaddr2: UserPtr<u32>,
        val: u32,
        val2: u32,
    ) -> usize {
        todo!()
    }
}

pub struct FutexFuture {
    word: UserPtr<u32>,
    val: u32,
}

impl FutexFuture {
    pub fn new(word: UserPtr<u32>, val: u32) -> Self {
        Self { word, val }
    }
}

impl Future for FutexFuture {
    type Output = u32;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        todo!("implement futex poll");
    }
}

/*

Futex是一个主要运行与用户空间的互斥锁, 用于竞争较少的情况
主要维护思想是: 先尝试在用户空间当中进行上锁, 如果上锁成功则直接使用
如果上锁失败, 再陷入内核当中插入到waitqueue当中进行等待
这样可以在竞争较小的情况下尽量的避免内核陷入

内核当中的sys_futex是用于维护上锁失败时的等待的

*/

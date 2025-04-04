use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::{
    future::poll_fn,
    task::{Poll, Waker},
    usize,
};

use array_init::array_init;
use ksync::cell::SyncUnsafeCell;
use lazy_static::lazy_static;

use super::{gettime::get_time, timer::get_sleep_block_limit_ticks};
use crate::{config::cpu::CPU_NUM, cpu::get_hartid, task::Task};

pub struct SleepInfo {
    waker: Waker,
    time: usize,
}

pub struct SleepManager {
    info: Option<SleepInfo>,
    queue: VecDeque<SleepInfo>,
}

impl SleepManager {
    pub fn new() -> Self {
        Self {
            info: None,
            queue: VecDeque::new(),
        }
    }
}

lazy_static! {
    pub static ref SLEEP_MANAGER: [SyncUnsafeCell<SleepManager>; CPU_NUM] =
        array_init(|_| SyncUnsafeCell::new(SleepManager::new()));
}

pub fn current_sleep_manager() -> &'static mut SleepManager {
    unsafe { &mut *SLEEP_MANAGER[get_hartid()].get() }
}

#[inline(always)]
fn check_time(current_time: usize, next_time: usize) -> bool {
    (current_time - next_time) as isize >= 0
}

impl SleepManager {
    pub fn sleep_handler(&mut self) {
        if let Some(info) = self.info.take() {
            let current_time = get_time();
            if check_time(current_time, info.time) {
                // sleep wake detected! try check if there are more tasks to wake
                info.waker.wake();
                while let Some(info) = self.queue.pop_front() {
                    if check_time(current_time, info.time) {
                        info.waker.wake();
                    } else {
                        self.info = Some(info);
                        break;
                    }
                }
            } else {
                self.info = Some(info);
            }
        }
    }
    pub fn push(&mut self, info: SleepInfo) {
        match self.info {
            None => self.info = Some(info),
            Some(_) => self.queue.push_back(info),
        }
    }
}

pub fn block_on_sleep(time: usize) {
    while !check_time(get_time(), time) {}
}

impl Task {
    pub async fn sleep(self: &Arc<Self>, interval: usize) {
        let time = get_time() + interval;
        if interval < get_sleep_block_limit_ticks() {
            block_on_sleep(time);
        } else {
            let waker = self.waker().as_ref().unwrap().clone();
            current_sleep_manager().push(SleepInfo { waker, time });
            poll_fn(move |_| match check_time(get_time(), time) {
                true => Poll::Ready(()),
                false => Poll::Pending,
            })
            .await;
        }
    }
}

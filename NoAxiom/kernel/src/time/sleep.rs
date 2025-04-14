use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::{
    future::poll_fn,
    task::{Poll, Waker},
    time::Duration,
    usize,
};

use array_init::array_init;
use ksync::cell::SyncUnsafeCell;
use lazy_static::lazy_static;

use super::gettime::{get_time, get_time_duration};
use crate::{config::cpu::CPU_NUM, cpu::get_hartid, task::Task};

pub struct SleepInfo {
    waker: Waker,
    time: Duration,
}

#[repr(align(64))]
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

// #[inline(always)]
// fn check_time(current_time: usize, next_time: usize) -> bool {
//     (current_time - next_time) as isize >= 0
// }

impl SleepManager {
    pub fn sleep_handler(&mut self) {
        // todo: impl this
        // do nothing

        // if let Some(info) = self.info.take() {
        //     let current_time = get_time();
        //     if check_time(current_time, info.time) {
        //         info.waker.wake();
        //         while let Some(info) = self.queue.pop_front() {
        //             if check_time(current_time, info.time) {
        //                 info.waker.wake();
        //             } else {
        //                 self.info = Some(info);
        //                 break;
        //             }
        //         }
        //     } else {
        //         self.info = Some(info);
        //     }
        // }
    }
    pub fn push(&mut self, info: SleepInfo) {
        match self.info {
            None => self.info = Some(info),
            Some(_) => self.queue.push_back(info),
        }
    }
}

// pub fn block_on_sleep(time: Duration) {
//     while !check_time(get_time(), time) {}
// }

impl Task {
    pub async fn sleep(self: &Arc<Self>, interval: Duration) {
        let time = get_time_duration() + interval;
        if interval < Duration::from_micros(500) {
            return;
            // block_on_sleep(time);
        } else {
            crate::sched::utils::yield_now().await;
            // let waker = self.waker().as_ref().unwrap().clone();
            // current_sleep_manager().push(SleepInfo { waker, time });
            // poll_fn(move |_| match check_time(get_time(), time) {
            //     true => Poll::Ready(()),
            //     false => Poll::Pending,
            // })
            // .await;
        }
    }
}

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::{task::Waker, usize};

use array_init::array_init;
use ksync::cell::SyncUnsafeCell;
use lazy_static::lazy_static;

use super::gettime::get_time;
use crate::{config::arch::CPU_NUM, cpu::get_hartid, sched::utils::suspend_now, task::Task};

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
    current_time as isize - next_time as isize >= 0
}

impl SleepManager {
    pub fn sleep_handler(&mut self) {
        if let Some(info) = self.info.take() {
            let current_time = get_time();
            if check_time(current_time, info.time) {
                // sleep wake detected! try check if there are more tasks to wake
                while let Some(info) = self.queue.pop_front() {
                    if check_time(current_time, info.time) {
                        info.waker.wake();
                    } else {
                        self.info = Some(info);
                        break;
                    }
                }
            } else {
                // restore the info
                self.info = Some(info);
            }
        }
    }
    pub fn push(&mut self, info: SleepInfo) {
        self.queue.push_back(info);
    }
}

impl Task {
    pub async fn sleep(self: &Arc<Self>, time: usize) {
        if time < 1024 {
            warn!("sleep time is too short, return immediately");
            return;
        }
        let waker = self.waker().as_ref().unwrap().clone();
        current_sleep_manager().push(SleepInfo { waker, time });
        suspend_now().await;
    }
}

//! reference: Phoenix
//! - time maneger: https://gitlab.eduxiji.net/educg-group-22026-2376550/T202418123993075-1053/-/blob/265a693df30be6e11d9b96a3466bd5a70cd0f6af/modules/timer/src/lib.rs
//! - interval timer: https://gitlab.eduxiji.net/educg-group-22026-2376550/T202418123993075-1053/-/blob/db7779fa607177c3ef5b16dd9d1a84da902f7cd7/kernel/src/task/signal.rs

use alloc::{
    boxed::Box,
    collections::binary_heap::BinaryHeap,
    sync::{Arc, Weak},
};
use core::{
    cmp::Reverse,
    sync::atomic::{AtomicUsize, Ordering},
    task::Waker,
    time::Duration,
};

use ksync::{mutex::SpinLock, Lazy};

use super::gettime::get_time_duration;
use crate::{
    include::time::{ITimerVal, ITIMER_REAL},
    signal::{
        sig_detail::SigDetail,
        sig_info::{SigCode, SigInfo},
        sig_num::SigNum,
    },
    task::Task,
};

/// A trait that defines the event to be triggered when a timer expires.
/// The TimerEvent trait requires a callback method to be implemented,
/// which will be called when the timer expires.
pub trait TimerEvent: Send + Sync {
    /// The callback method to be called when the timer expires.
    /// This method consumes the event data and optionally returns a new timer.
    ///
    /// # Returns
    /// An optional Timer object that can be used to schedule another timer.
    fn callback(self: Box<Self>) -> Option<Timer>;
}

/// Represents a timer with an expiration time and associated event data.
/// The Timer structure contains the expiration time and the data required
/// to handle the event when the timer expires.
pub struct Timer {
    /// The expiration time of the timer.
    /// This indicates when the timer is set to trigger.
    pub expire: Duration,

    /// A boxed dynamic trait object that implements the TimerEvent trait.
    /// This allows different types of events to be associated with the timer.
    pub data: Box<dyn TimerEvent>,
}

impl Timer {
    pub fn new(expire: Duration, data: Box<dyn TimerEvent>) -> Self {
        Self { expire, data }
    }

    pub fn new_waker_timer(expire: Duration, waker: Waker) -> Self {
        struct WakerData {
            waker: Waker,
        }
        impl TimerEvent for WakerData {
            fn callback(self: Box<Self>) -> Option<Timer> {
                self.waker.wake();
                None
            }
        }
        Self::new(expire, Box::new(WakerData { waker }))
    }

    fn callback(self) -> Option<Timer> {
        self.data.callback()
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.expire.cmp(&other.expire)
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Timer {}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.expire == other.expire
    }
}

/// `TimerManager` is responsible for managing all the timers in the system.
/// It uses a thread-safe lock to protect a priority queue (binary heap) that
/// stores the timers. The timers are stored in a `BinaryHeap` with their
/// expiration times wrapped in `Reverse` to create a min-heap, ensuring that
/// the timer with the earliest expiration time is at the top.
pub struct TimerManager {
    /// A priority queue to store the timers. The queue is protected by a spin
    /// lock to ensure thread-safe access. The timers are wrapped in
    /// `Reverse` to maintain a min-heap.
    timers: SpinLock<BinaryHeap<Reverse<Timer>>>,
}

impl TimerManager {
    fn new() -> Self {
        Self {
            timers: SpinLock::new(BinaryHeap::new()),
        }
    }

    pub fn add_timer(&self, timer: Timer) {
        trace!("add new timer, next expiration {:?}", timer.expire);
        self.timers.lock().push(Reverse(timer));
    }

    pub fn check(&self) {
        let mut timers = self.timers.lock();
        while let Some(timer) = timers.peek() {
            let current_time = get_time_duration();
            if current_time >= timer.0.expire {
                trace!("timers len {}", timers.len());
                trace!(
                    "[Timer Manager] there is a timer expired, current:{:?}, expire:{:?}",
                    current_time, timer.0.expire
                );
                let timer = timers.pop().unwrap().0;
                if let Some(new_timer) = timer.callback() {
                    timers.push(Reverse(new_timer));
                }
            } else {
                break;
            }
        }
    }
}

pub static TIMER_MANAGER: Lazy<TimerManager> = Lazy::new(TimerManager::new);

pub fn timer_handler() {
    TIMER_MANAGER.check();
}

struct TimerIdAllocator(AtomicUsize);
impl TimerIdAllocator {
    pub const fn new() -> Self {
        // 0 is reserved for invalid timer id
        // hence we start from 1
        Self(AtomicUsize::new(1))
    }
    pub fn alloc(&self) -> usize {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}
static TIMER_ID_ALLOCATOR: TimerIdAllocator = TimerIdAllocator::new();

pub type ITimerID = usize;

#[derive(Debug, Clone, Copy)]
pub struct ITimer {
    pub interval: Duration,
    pub expire: Duration,
    pub timer_id: ITimerID,
}
impl ITimer {
    pub const ZERO: Self = Self::new_bare();
    pub const fn new_bare() -> Self {
        Self {
            interval: Duration::ZERO,
            expire: Duration::ZERO,
            timer_id: 0,
        }
    }
    pub fn is_disarmed(&self) -> bool {
        self.expire == Duration::ZERO
    }
    pub fn register(itimer_val: &ITimerVal) -> Self {
        let timer_id = TIMER_ID_ALLOCATOR.alloc();
        if itimer_val.it_value.is_zero() {
            Self {
                interval: itimer_val.it_interval.into(),
                expire: Duration::ZERO,
                timer_id,
            }
        } else {
            Self {
                interval: itimer_val.it_interval.into(),
                expire: get_time_duration() + itimer_val.it_value.into(),
                timer_id,
            }
        }
    }
    pub fn into_itimer_val(&self) -> ITimerVal {
        ITimerVal {
            it_interval: self.interval.into(),
            it_value: self.expire.saturating_sub(get_time_duration()).into(),
        }
    }
}

pub struct ITimerManager {
    inner: [ITimer; 3],
}
impl ITimerManager {
    pub const fn new() -> Self {
        Self {
            inner: [ITimer::ZERO; 3],
        }
    }
    pub fn get(&self, which: usize) -> &ITimer {
        &self.inner[which]
    }
    pub fn get_mut(&mut self, which: usize) -> &mut ITimer {
        &mut self.inner[which]
    }
    pub fn set(&mut self, which: usize, itimer: ITimer) {
        self.inner[which] = itimer;
    }
}
impl Default for ITimerManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ITimerReal {
    task: Weak<Task>,
    timer_id: ITimerID,
}
impl ITimerReal {
    pub fn new(task: &Arc<Task>, timer_id: ITimerID) -> Self {
        Self {
            task: Arc::downgrade(task),
            timer_id,
        }
    }
}

impl TimerEvent for ITimerReal {
    fn callback(self: Box<Self>) -> Option<Timer> {
        if let Some(task) = self.task.upgrade() {
            let mut manager = task.itimer();
            let real = manager.get_mut(ITIMER_REAL);
            if real.timer_id != self.timer_id {
                // current timer is old
                return None;
            }
            task.recv_siginfo(
                SigInfo {
                    signo: SigNum::SIGALRM.into(),
                    code: SigCode::Kernel,
                    errno: 0,
                    detail: SigDetail::None,
                },
                false,
            );
            if real.interval == Duration::ZERO {
                // current timer should only be triggered once
                return None;
            }
            real.expire = get_time_duration() + real.interval;
            Some(Timer::new(real.expire, self))
        } else {
            None
        }
    }
}

//! [WIP] The EventWaitQueue
//!
//! definition: Let current process wait for its interested events. When call
//! `sleep`, the process will be blocked (we will schedule new work for current
//! hart) until the EventWaitQueue call `wake_up` wakes it up.
//!
//! e.g. for socket I/O

use alloc::{sync::Arc, vec::Vec};
use core::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use ksync::mutex::SpinLock;

use crate::{
    sched::utils::{suspend_now, yield_now},
    syscall::SysResult,
    task::Task,
};

/// for ProcessManager
pub trait AwakenedProcessManager {
    /// Check whether the lock holding count of the current process is greater
    /// than the maximum value
    fn preempt_check(max: usize) -> bool;

    /// Indicate that the current process is permanently asleep, but the task of
    /// initiating scheduling should be completed by the caller
    ///
    /// ## Attention
    ///
    /// - Before entering the current function, cannot hold a lock on
    ///   ScheduleInfo
    ///
    /// - Before entering the current function, interrupt must be turned off
    ///
    /// - After entering the current function, we should schedule manually
    fn sleep(interruptable: bool) -> SysResult<()>;

    /// Wake up the process
    fn wake_up(pcb: Arc<Task>) -> SysResult<()>;

    fn current_process() -> Arc<Task>;
}

#[derive(Default)]
struct TmpProcessManager;
impl AwakenedProcessManager for TmpProcessManager {
    fn preempt_check(_max: usize) -> bool {
        unreachable!()
    }
    fn sleep(_interruptable: bool) -> SysResult<()> {
        unreachable!()
    }
    fn wake_up(_pcb: Arc<Task>) -> SysResult<()> {
        unreachable!()
    }
    fn current_process() -> Arc<Task> {
        unreachable!()
    }
}

type Manager = TmpProcessManager;

enum EventState {
    Created,
    Waiting,
    Accepted,
}

/// ## The `Event`
///
///
/// ### Usage
///
/// ```rust
/// // some where define the event_wait_queue
/// let async_driver_wait_queue = Arc::new(SpinLock::new(EventWaitQueue::new()));
///
/// // process A:
/// let event_id = alloc();
/// let event = Event::new(event_id, async_driver_wait_queue.clone()); // todo: too many parameters!
/// send_read_request(event_id);
/// event.await; // wait for the event
/// handle_read_response(event_id);
///
/// // process B(like interrupt handler):
/// async_driver_wait_queue.lock().wake_up(event_id);
/// ```
pub struct Event {
    pub id: usize,
    wait_queue: Arc<SpinLock<EventWaitQueue>>,
    state: UnsafeCell<EventState>,
}

impl Event {
    pub fn new(id: usize, wait_queue: Arc<SpinLock<EventWaitQueue>>) -> Self {
        Self {
            id,
            wait_queue,
            state: UnsafeCell::new(EventState::Created),
        }
    }
}

impl Future for Event {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let state = unsafe { &mut *self.state.get() };
        match *state {
            EventState::Created => {
                *state = EventState::Waiting;
                let mut queue = self.wait_queue.lock();
                queue.sleep(self.id, cx.waker().clone()).unwrap();
                Poll::Pending
            }
            EventState::Waiting => Poll::Pending,
            EventState::Accepted => Poll::Ready(()),
        }
    }
}

/// ## The EventWaitQueue
///
/// ### Usage
///
/// ```rust
/// 
/// // process A:
/// let event_queue = EventWaitQueue::new();
/// let events = Event(0o01010);               // interested in the 1st and 3th events
/// event_queue.sleep_schedule(events).await?; // sleep and schedule until wake_up
/// //...do something
///
/// // process B:
/// let wake_count = event_queue.wake_up(events)?;
/// ```
pub struct EventWaitQueue {
    /// the list of tasks waiting for this event
    inner: Vec<(usize, Waker)>,
}

impl EventWaitQueue {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Let current process wait for the event
    pub fn sleep(&mut self, id: usize, waker: Waker) -> SysResult<()> {
        Manager::preempt_check(0);
        self.inner.push((id, waker));
        Manager::sleep(true)?;
        Ok(())
    }

    /// Wake up the process with event `id`
    pub fn wake_up(&mut self, id: usize) -> SysResult<()> {
        self.inner.retain(|(event_id, waker)| {
            if *event_id == id {
                waker.wake_by_ref();
                false
            } else {
                true
            }
        });
        Ok(())
    }

    /// Wake up all process waiting for the event
    pub fn wake_up_all(&mut self) -> SysResult<()> {
        self.inner.retain(|(_, waker)| {
            waker.wake_by_ref();
            false
        });
        Ok(())
    }
}

//! [WIP] The EventWaitQueue
//!
//! definition: Let current process wait for its interested events. When call
//! `sleep`, the process will be blocked (we will schedule new work for current
//! hart) until the EventWaitQueue call `wake_up` wakes it up.
//!
//! e.g. for socket I/O

use alloc::{sync::Arc, vec::Vec};

use ksync::mutex::SpinLock;

use crate::{sched::utils::yield_now, syscall::SysResult, task::Task};

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

/// ## The `Event`
///
/// we use bit to represent the interested events
///
/// ### Usage
///
/// ```rust
/// let events = Event(0o01010); // interested in the 1st and 3th events
/// let events = Event(0o11111); // interested in all the events
/// ```
pub struct Event(usize);

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
    inner: SpinLock<Vec<(Event, Arc<Task>)>>,
}

impl EventWaitQueue {
    pub fn new() -> Self {
        Self {
            inner: SpinLock::new(Vec::new()),
        }
    }

    /// Let current process wait for the events, schedule
    pub async fn sleep_schedule(&self, events: Event) -> SysResult<()> {
        Manager::preempt_check(0);
        let mut wait_queue = self.inner.lock();
        wait_queue.push((events, Manager::current_process()));
        Manager::sleep(true)?;
        yield_now().await;
        Ok(())
    }

    /// Let current process wait for the events without schedule
    pub fn sleep(&self, events: Event) -> SysResult<()> {
        Manager::preempt_check(0);
        let mut wait_queue = self.inner.lock();
        wait_queue.push((events, Manager::current_process()));
        Manager::sleep(true)?;
        Ok(())
    }

    /// Wake up the process at **all** the events
    pub fn wake_up(&self, events: Event) -> SysResult<usize> {
        let mut wake_count = 0;
        let mut wait_queue = self.inner.lock();
        wait_queue.retain(|(event, task)| {
            if event.0 == events.0 {
                wake_count += 1;
                Manager::wake_up(task.clone()).unwrap();
                false
            } else {
                true
            }
        });
        Ok(wake_count)
    }

    pub fn wake_up_any(&self, events: Event) -> SysResult<usize> {
        let mut wake_count = 0;
        let mut wait_queue = self.inner.lock();
        wait_queue.retain(|(event, task)| {
            if event.0 & events.0 != 0 {
                wake_count += 1;
                Manager::wake_up(task.clone()).unwrap();
                false
            } else {
                true
            }
        });
        Ok(wake_count)
    }

    pub fn wake_up_all(&self, events: Event) -> SysResult<usize> {
        self.wake_up_any(Event(usize::MAX))
    }
}

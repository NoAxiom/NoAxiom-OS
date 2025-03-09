//! [WIP] The Event and EventWaitQueue
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

use crate::{include::result::Errno, syscall::SysResult};

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

    // // Wake up the process
    // fn wake_up(pcb: Arc<Task>) -> SysResult<()>;

    // fn current_process() -> Arc<Task>;
}

/// The `Req` trait is used to define the type of the request
pub trait Req: PartialEq + Clone {}
impl Req for usize {}
impl Req for u64 {}
impl Req for u32 {}
impl Req for u8 {}

/// The `Res` trait is used to define the type of the response
pub trait Res: Copy {}
impl Res for usize {}
impl Res for u64 {}
impl Res for u32 {}
impl Res for u8 {}
impl Res for () {}

#[derive(Default)]
struct TmpProcessManager;
impl AwakenedProcessManager for TmpProcessManager {
    fn preempt_check(_max: usize) -> bool {
        unreachable!()
    }
    fn sleep(_interruptable: bool) -> SysResult<()> {
        unreachable!()
    }
    // fn wake_up(_pcb: Arc<Task>) -> SysResult<()> {
    //     unreachable!()
    // }
    // fn current_process() -> Arc<Task> {
    //     unreachable!()
    // }
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
/// ### Usage Example 1
///
/// ```rust
/// // some where define the wait_queue for sys_exit4
/// lazy_static! {
///     static ref EXIT_WAIT_QUEUE: ...
/// }
/// struct SigEvent;
/// impl Event for SigEvent { ... }
///
/// // process A(e.g. sys_wait4):
/// async fn sys_wait4() {
///     // ...
///     let event = SigEvent::new(self.pid);
///     event.await; // wait for the event "SIG_CHILD"
///     //...
/// }
///
/// // process B(may be in child process):
/// fn sys_exit() {
///     // ...
///     SigEvent::wake_up(self.parent.pid); // wake up the process in the queue
///     // ...
/// }
/// ```
///    
///     
/// ### Usage Example 2
///
/// ```rust
/// // some where define the wait_queue for Driver
/// lazy_static! {
///     static ref DRIVER_WAIT_QUEUE: ...
/// }
/// struct AsyncDriverEvent;
/// impl Event for AsyncDriverEvent { ... }
///
/// // process A(e.g. async_read):
/// async fn read() {
///     // ...
///     let event = AsyncDriverEvent::new(block_id);
///     let res = event.await; // wait for the event "read"
///     //...
/// }
///
/// // process B(may be in device interrupt handler):
/// fn handler() {
///     // ...
///     let block_id = AsyncDriverEvent::get_req().unwrap();
///     let data = get_data(block_id);
///     AsyncDriverEvent::wake_up(block_id, data);
///     // ...
/// }
/// ```
pub trait Event {
    type Q: Req;
    type T: Res;

    /// You should have a static `wait_queue` for each kind of event
    fn wait_queue() -> Arc<SpinLock<EventWaitQueue<Self::Q, Self::T>>>;

    fn new(req: Self::Q) -> EventInner<Self::Q, Self::T> {
        EventInner::new(req, Self::wait_queue())
    }

    fn wake_up(req: &Self::Q, res: Self::T) -> SysResult<()> {
        Self::wait_queue().lock().wake_up(req, res)
    }

    fn get_req() -> Option<Self::Q> {
        Self::wait_queue().lock().get_req().cloned()
    }
}

pub struct EventInner<Q: Req, T: Res> {
    req: Q,
    wait_queue: Arc<SpinLock<EventWaitQueue<Q, T>>>,
    state: UnsafeCell<EventState>,
}

impl<Q: Req, T: Res> EventInner<Q, T> {
    pub fn new(req: Q, wait_queue: Arc<SpinLock<EventWaitQueue<Q, T>>>) -> Self {
        Self {
            req,
            wait_queue,
            state: UnsafeCell::new(EventState::Created),
        }
    }

    fn set_state(&self, state: EventState) {
        unsafe {
            *self.state.get() = state;
        }
    }
}

impl<Q: Req, T: Res> Future for EventInner<Q, T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let state = unsafe { &mut *self.state.get() };
        match *state {
            EventState::Created => {
                self.set_state(EventState::Waiting);
                let mut queue = self.wait_queue.lock();
                queue.sleep(self.req.clone(), cx.waker().clone()).unwrap();
                Poll::Pending
            }
            EventState::Waiting => Poll::Pending, // reset waker?
            EventState::Accepted => {
                let res = self.wait_queue.lock().get_res(&self.req).unwrap();
                Poll::Ready(res)
            }
        }
    }
}

/// ## The EventWaitQueue
/// the event inner wait queue
pub struct EventWaitQueue<Q: Req, T: Res> {
    /// the list of tasks waiting for this event
    inner: Vec<(Q, Option<T>, Waker)>,
}

impl<Q: Req, T: Res> EventWaitQueue<Q, T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Let current process wait for the event
    pub fn sleep(&mut self, req: Q, waker: Waker) -> SysResult<()> {
        Manager::preempt_check(0);
        self.inner.push((req, None, waker));
        Manager::sleep(true)?;
        Ok(())
    }

    /// Wake up the `req` process with result `res`
    pub fn wake_up(&mut self, req: &Q, res: T) -> SysResult<()> {
        for (event_req, empty_res, waker) in self.inner.iter_mut() {
            if event_req == req {
                assert!(empty_res.is_none());
                *empty_res = Some(res);
                waker.wake_by_ref();
                return Ok(());
            }
        }
        Err(Errno::ENOENT)
    }

    /// for handler to get the req
    pub fn get_req(&self) -> Option<&Q> {
        self.inner.first().map(|(req, ..)| req)
    }

    pub fn get_res(&mut self, req: &Q) -> Option<T> {
        let mut res = None;
        self.inner.retain(|(event_req, event_res, _)| {
            if event_req == req {
                res = *event_res;
                false
            } else {
                true
            }
        });
        res
    }
}

//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::{sync::Weak, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use arch::{Arch, ArchAsm, ArchInt};
use array_init::array_init;
use async_task::{Runnable, ScheduleInfo};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{check_no_lock, SpinLock},
};

use super::{
    sched_entity::SchedEntity,
    vsched::{MulticoreRuntime, MulticoreSchedInfo, MulticoreScheduler, Runtime, ScheduleOrder},
};
use crate::{
    config::{arch::CPU_NUM, sched::LOAD_BALANCE_TICKS},
    cpu::get_hartid,
    sched::runtime::RUNTIME,
    task::{status::TaskStatus, Task},
    time::{gettime::get_time, sleep::current_sleep_manager},
    trap::ipi::{send_ipi, IpiType},
};

pub struct SchedInfo {
    pub sched_entity: SchedEntity,
    /// the hartid that the task should be running on
    pub hartid: AtomicUsize,
    pub task: Option<Weak<Task>>,
}
impl SchedInfo {
    pub fn new(sched_entity: SchedEntity, hartid: usize, task: Option<Weak<Task>>) -> Self {
        Self {
            sched_entity,
            hartid: AtomicUsize::new(hartid),
            task,
        }
    }
}

impl MulticoreSchedInfo for SchedInfo {
    fn set_hartid(&self, hartid: usize) {
        self.hartid.store(hartid, Ordering::Release);
    }
    fn hartid(&self) -> usize {
        self.hartid.load(Ordering::Acquire)
    }
}

struct RunnableMailbox<T> {
    pub valid: AtomicBool,
    pub mailbox: SpinLock<Vec<T>>,
}
impl<T> RunnableMailbox<T> {
    pub fn new() -> Self {
        Self {
            valid: AtomicBool::new(false),
            mailbox: SpinLock::new(Vec::new()),
        }
    }
}

pub struct NoAxiomRuntime<T, R>
where
    T: MulticoreScheduler<R>,
    R: MulticoreSchedInfo,
{
    /// global task mailbox
    mailbox: [RunnableMailbox<Runnable<R>>; CPU_NUM],

    /// use cpu mask to pass request
    sched_req: AtomicUsize,

    /// the load sum of all cores
    all_load: AtomicUsize,

    /// last contribution time
    last_handle_time: [SyncUnsafeCell<usize>; CPU_NUM],

    /// last request time
    last_request_time: [SyncUnsafeCell<usize>; CPU_NUM],

    /// scheduler for each core
    scheduler: [SyncUnsafeCell<T>; CPU_NUM],
}

impl<T, R> NoAxiomRuntime<T, R>
where
    T: MulticoreScheduler<R>,
    R: MulticoreSchedInfo,
{
    fn current_scheduler(&self) -> &mut T {
        unsafe { &mut *self.scheduler[get_hartid()].get() }
    }
    fn pop(&self) -> Option<Runnable<R>> {
        self.current_scheduler().pop(ScheduleOrder::UrgentFirst)
    }
    fn pop_normal_first(&self) -> Option<Runnable<R>> {
        self.current_scheduler().pop(ScheduleOrder::NormalFirst)
    }
    fn set_sched_req(&self) {
        let mask = 1 << get_hartid();
        self.sched_req.fetch_or(mask, Ordering::AcqRel);
        self.set_last_request_time();
    }
    fn get_load(&self) -> usize {
        self.all_load.load(Ordering::Acquire)
    }
    fn last_handle_time(&self) -> usize {
        unsafe { *self.last_handle_time[get_hartid()].get() }
    }
    fn last_request_time(&self) -> usize {
        unsafe { *self.last_request_time[get_hartid()].get() }
    }
    fn set_last_handle_time(&self) {
        *&mut unsafe { *self.last_handle_time[get_hartid()].get() } = get_time();
    }
    fn set_last_request_time(&self) {
        *&mut unsafe { *self.last_request_time[get_hartid()].get() } = get_time();
    }
    fn current_is_overload(&self) -> bool {
        self.current_scheduler().is_overload(RUNTIME.get_load())
    }
    fn current_is_underload(&self) -> bool {
        self.current_scheduler().is_underload(RUNTIME.get_load())
    }
    fn current_can_resp_sched_req(&self) -> bool {
        let is_timeup = get_time() - self.last_handle_time() > LOAD_BALANCE_TICKS;
        is_timeup && self.current_is_overload()
    }
    fn current_should_set_sched_req(&self) -> bool {
        let is_timeup = get_time() - self.last_request_time() > LOAD_BALANCE_TICKS;
        is_timeup && self.current_is_underload()
    }

    fn try_resp_sched_req(&self) {
        if self.sched_req.load(Ordering::Acquire) == 0 {
            return;
        }
        trace!("[try_respond_sched_req] begin!");
        let cur_hartid = get_hartid();
        // todo: use ticketed behaviour instead of from zero
        for i in 0..CPU_NUM {
            if i == cur_hartid {
                continue;
            }
            let mask = 1 << i;
            let val = self.sched_req.fetch_and(!mask, Ordering::AcqRel);
            if val & mask != 0 {
                // request detected, now push tasks
                let mut mailbox = self.mailbox[i].mailbox.lock();
                // the overall load will change when we pop tasks
                // so save the previous load first
                let all_load = RUNTIME.get_load();
                while self.current_scheduler().is_overload(all_load) {
                    if let Some(runnable) = self.pop_normal_first() {
                        runnable.metadata().set_hartid(i);
                        mailbox.push(runnable);
                    } else {
                        error!("[try_respond_sched_req] break from loop");
                        break;
                    }
                }
                warn!("[load_balance] move: {} -> {}", cur_hartid, i);
                self.set_last_handle_time();
                self.mailbox[i].valid.store(true, Ordering::Release);
                drop(mailbox);
                send_ipi(i, IpiType::LoadBalance);
                return;
            }
        }
        error!("[load_balance] failed");
    }

    /// when other core detect a load imbalance, it will send a IPI to this core
    /// and then the current core will enter this function to fetch global tasks
    pub fn handle_mailbox(&self) {
        let hartid = get_hartid();
        if !self.mailbox[hartid].valid.load(Ordering::Acquire) {
            return;
        }
        trace!("[handle_mailbox] begin");
        let mut mailbox = self.mailbox[hartid].mailbox.lock();
        let local = self.current_scheduler();
        while let Some(runnable) = mailbox.pop() {
            // fixme: is urgent_queue correct?
            local.push_urgent(runnable);
        }
        self.mailbox[hartid].valid.store(false, Ordering::Release);
        drop(mailbox);
    }

    /// push one waker into other's mailbox
    /// this function is called when a task is woken up from other core
    fn push_one_to_mailbox(&self, hartid: usize, runnable: Runnable<R>) {
        let mut mailbox = self.mailbox[hartid].mailbox.lock();
        self.mailbox[hartid].valid.store(true, Ordering::Release);
        mailbox.push(runnable);
    }
}

impl<T, R> MulticoreRuntime<T, R> for NoAxiomRuntime<T, R>
where
    Self: Runtime<T, R>,
    T: MulticoreScheduler<R>,
    R: MulticoreSchedInfo,
{
    fn add_load(&self, load: usize) {
        self.all_load.fetch_add(load, Ordering::AcqRel);
    }
    fn sub_load(&self, load: usize) {
        self.all_load.fetch_sub(load, Ordering::AcqRel);
    }
}

impl<T> Runtime<T, SchedInfo> for NoAxiomRuntime<T, SchedInfo>
where
    T: MulticoreScheduler<SchedInfo>,
{
    fn new() -> Self {
        Self {
            mailbox: array_init(|_| RunnableMailbox::new()),
            sched_req: AtomicUsize::new(0),
            all_load: AtomicUsize::new(0),
            last_handle_time: array_init(|_| SyncUnsafeCell::new(0)),
            last_request_time: array_init(|_| SyncUnsafeCell::new(0)),
            scheduler: array_init(|_| SyncUnsafeCell::new(T::default())),
        }
    }

    fn schedule(&self, runnable: Runnable<SchedInfo>, info: ScheduleInfo) {
        let woken_hartid = runnable.metadata().hartid();
        if let Some(task) = runnable.metadata().task.as_ref() {
            if let Some(task) = task.upgrade() {
                let old_status = task.swap_status(TaskStatus::Runnable);
                if old_status == TaskStatus::Runnable && !info.woken_while_running {
                    error!("task {} is already runnable", task.tid());
                    return;
                }
            }
        }
        #[cfg(feature = "multicore")]
        if woken_hartid == get_hartid() {
            trace!(
                "[schedule] push into local scheduler, tid: {}",
                runnable.metadata().sched_entity.tid
            );
            self.current_scheduler().push_with_info(runnable, info);
        } else {
            info!(
                "[schedule] push to other's mailbox, tid: {}",
                runnable.metadata().sched_entity.tid
            );
            self.push_one_to_mailbox(woken_hartid, runnable);
        }
        #[cfg(not(feature = "multicore"))]
        self.current_scheduler().push_with_info(runnable, info);
    }

    fn run(&self) {
        #[cfg(feature = "multicore")]
        self.handle_mailbox();
        current_sleep_manager().sleep_handler();
        assert!(check_no_lock(), "LOCK IS NOT RELEASED!!!");
        if let Some(runnable) = self.pop() {
            runnable.run();
            #[cfg(feature = "multicore")]
            if self.current_can_resp_sched_req() {
                self.try_resp_sched_req();
            } else if self.current_should_set_sched_req() {
                trace!("[set_sched_req] current is underload");
                self.set_sched_req();
            }
        } else {
            #[cfg(feature = "multicore")]
            if self.current_should_set_sched_req() {
                trace!("[set_sched_req] empty queue");
                self.set_sched_req();
                assert!(Arch::is_interrupt_enabled());
                Arch::set_idle();
                return;
            }
        }
    }
}

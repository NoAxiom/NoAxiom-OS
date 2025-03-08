//! ## async executor
//! - [`spawn_raw`] to add a task
//! - [`run`] to run next task

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use arch::{Arch, ArchAsm, ArchInt};
use array_init::array_init;
use async_task::{Runnable, ScheduleInfo};
use ksync::{cell::SyncUnsafeCell, mutex::SpinLock};
use lazy_static::lazy_static;

use super::{cfs::CFS, sched_entity::SchedEntity, scheduler::Scheduler};
use crate::{
    config::{arch::CPU_NUM, sched::LOAD_BALANCE_TICKS},
    cpu::get_hartid,
    time::gettime::get_time,
    trap::ipi::{send_ipi, IpiType},
};

pub struct TaskScheduleInfo {
    pub sched_entity: SchedEntity,
    /// the hartid that the task should be running on
    pub hartid: AtomicUsize,
}
impl TaskScheduleInfo {
    pub fn new(sched_entity: SchedEntity, hartid: usize) -> Self {
        Self {
            sched_entity,
            hartid: AtomicUsize::new(hartid),
        }
    }
    pub fn set_hartid(&self, hartid: usize) {
        self.hartid.store(hartid, Ordering::SeqCst);
    }
    pub fn hartid(&self) -> usize {
        self.hartid.load(Ordering::SeqCst)
    }
}

pub type RunnableTask = Runnable<TaskScheduleInfo>;

struct RunnableMailbox {
    pub valid: AtomicBool,
    pub mailbox: SpinLock<Vec<RunnableTask>>,
}
impl RunnableMailbox {
    pub fn new() -> Self {
        Self {
            valid: AtomicBool::new(false),
            mailbox: SpinLock::new(Vec::new()),
        }
    }
}

pub struct Runtime<T: Scheduler> {
    /// global task queue
    mailbox: [RunnableMailbox; CPU_NUM],

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

impl<T> Runtime<T>
where
    T: Scheduler,
{
    pub fn new() -> Self {
        Self {
            mailbox: array_init(|_| RunnableMailbox::new()),
            sched_req: AtomicUsize::new(0),
            all_load: AtomicUsize::new(0),
            last_handle_time: array_init(|_| SyncUnsafeCell::new(0)),
            last_request_time: array_init(|_| SyncUnsafeCell::new(0)),
            scheduler: array_init(|_| SyncUnsafeCell::new(T::default())),
        }
    }

    pub fn current_scheduler(&self) -> &mut T {
        unsafe { &mut *self.scheduler[get_hartid()].get() }
    }

    pub fn schedule(&self, runnable: RunnableTask, info: ScheduleInfo) {
        let woken_hartid = runnable.metadata().hartid();
        if woken_hartid == get_hartid() {
            self.current_scheduler().push_with_info(runnable, info);
        } else {
            self.push_one_to_mailbox(woken_hartid, runnable);
        }
    }
    pub fn pop(&self) -> Option<RunnableTask> {
        self.current_scheduler().pop()
    }

    pub fn set_sched_req(&self) {
        let mask = 1 << get_hartid();
        self.sched_req.fetch_or(mask, Ordering::SeqCst);
        self.set_last_request_time();
    }

    pub fn get_load(&self) -> usize {
        RUNTIME.all_load.load(core::sync::atomic::Ordering::SeqCst)
    }
    pub fn add_load(&self, load: usize) {
        self.all_load.fetch_add(load, Ordering::SeqCst);
    }
    pub fn sub_load(&self, load: usize) {
        self.all_load.fetch_sub(load, Ordering::SeqCst);
    }

    pub fn last_handle_time(&self) -> usize {
        unsafe { *self.last_handle_time[get_hartid()].get() }
    }
    pub fn last_request_time(&self) -> usize {
        unsafe { *self.last_request_time[get_hartid()].get() }
    }
    pub fn set_last_handle_time(&self) {
        *&mut unsafe { *self.last_handle_time[get_hartid()].get() } = get_time();
    }
    pub fn set_last_request_time(&self) {
        *&mut unsafe { *self.last_request_time[get_hartid()].get() } = get_time();
    }

    pub fn current_is_overload(&self) -> bool {
        self.current_scheduler().is_overload(RUNTIME.get_load())
    }
    pub fn current_is_underload(&self) -> bool {
        self.current_scheduler().is_underload(RUNTIME.get_load())
    }
    pub fn current_can_resp_sched_req(&self) -> bool {
        let is_timeup = get_time() - self.last_handle_time() > LOAD_BALANCE_TICKS;
        is_timeup && self.current_is_overload()
    }
    pub fn current_should_set_sched_req(&self) -> bool {
        let is_timeup = get_time() - self.last_request_time() > LOAD_BALANCE_TICKS;
        is_timeup && self.current_is_underload()
    }

    pub fn try_resp_sched_req(&self) {
        if self.sched_req.load(Ordering::SeqCst) == 0 {
            return;
        }
        trace!("[try_respond_sched_req] begin!");
        let cur_hartid = get_hartid();
        for i in 0..CPU_NUM {
            if i == cur_hartid {
                continue;
            }
            let mask = 1 << i;
            let val = self.sched_req.fetch_and(!mask, Ordering::SeqCst);
            if val & mask != 0 {
                // request detected, now push tasks
                let mut mailbox = self.mailbox[i].mailbox.lock();
                // the overall load will change when we pop tasks
                // so save the previous load first
                let all_load = RUNTIME.get_load();
                while self.current_scheduler().is_overload(all_load) {
                    if let Some(runnable) = self.pop() {
                        runnable.metadata().set_hartid(i);
                        mailbox.push(runnable);
                    } else {
                        error!("[try_respond_sched_req] break from loop");
                        break;
                    }
                }
                let tid_queue: Vec<usize> = mailbox
                    .iter()
                    .map(|task| task.metadata().sched_entity.tid)
                    .collect();
                warn!("[load_balance] move: {} -> {}", cur_hartid, i);
                warn!("[load_balance] tid_queue: {:?}", tid_queue);
                self.set_last_handle_time();
                self.mailbox[i].valid.store(true, Ordering::SeqCst);
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
        if !self.mailbox[hartid].valid.load(Ordering::SeqCst) {
            return;
        }
        debug!("[handle_mailbox] begin");
        let mut mailbox = self.mailbox[hartid].mailbox.lock();
        let local = self.current_scheduler();
        while let Some(task) = mailbox.pop() {
            debug!("[handle_mailbox] push tid: {}", task.metadata().sched_entity.tid);
            local.push_normal(task);
        }
        self.mailbox[hartid].valid.store(false, Ordering::SeqCst);
        drop(mailbox);
    }

    /// push one task into other's mailbox
    /// this function is called when a task is woken up from other core
    pub fn push_one_to_mailbox(&self, hartid: usize, task: RunnableTask) {
        let mut mailbox = self.mailbox[hartid].mailbox.lock();
        self.mailbox[hartid].valid.store(true, Ordering::SeqCst);
        mailbox.push(task);
    }

    /// Pop a task and run it
    pub fn run(&self) {
        self.handle_mailbox();
        if let Some(runnable) = self.pop() {
            runnable.run();
            #[cfg(feature = "multicore")]
            if self.current_can_resp_sched_req() {
                self.try_resp_sched_req();
            } else if self.current_should_set_sched_req() {
                info!("[set_sched_req] current is underload");
                self.set_sched_req();
            }
        } else {
            #[cfg(feature = "multicore")]
            if self.current_should_set_sched_req() {
                info!("[set_sched_req] empty queue");
                self.set_sched_req();
                assert!(Arch::is_interrupt_enabled());
                Arch::set_idle();
                return;
            }
        }
    }
}

// TODO: add muticore support
lazy_static! {
    pub static ref RUNTIME: Runtime<CFS> = Runtime::new();
}

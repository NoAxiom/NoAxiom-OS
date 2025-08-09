//! # Task

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{
    intrinsics::unlikely,
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicU32, AtomicUsize},
    task::Waker,
};

use arch::{Arch, ArchInt, TrapContext};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
};

use super::{
    manager::ThreadGroup,
    pcb::PCB,
    taskid::{TidTracer, PGID, PID, TGID, TID},
    tcb::TCB,
};
use crate::{
    fs::{fdtable::FdTable, vfs::basic::dentry::Dentry},
    include::{process::TaskFlags, sched::CpuMask},
    mm::{memory_set::MemorySet, user_ptr::UserPtr},
    sched::sched_entity::{SchedEntity, SchedPrio},
    signal::{sig_action::SigActionList, sig_set::SigMask, sig_stack::UContext},
    task::futex::FutexQueue,
    time::{time_info::TimeInfo, timer::ITimerManager},
};

/// shared between threads
pub(super) type SharedMut<T> = Arc<SpinLock<T>>;
pub(super) struct Shared<T>(PhantomData<T>);
impl<T> Shared<T> {
    pub fn new(data: T) -> SharedMut<T> {
        Arc::new(SpinLock::new(data))
    }
}
impl Shared<usize> {
    pub fn new_atomic(data: usize) -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(data))
    }
}

/// mutable resources mostly used in current thread
/// but, it could be accessed by other threads through process manager
/// so lock it with spinlock
/// p.s. we can also use atomic if the data is small enough
pub(super) type Mutable<T> = SpinLock<T>;

/// read-only resources, could be shared safely through threads
pub(super) type Immutable<T> = T;

/// only used in current thread, mutable resources without lock
/// SAFETY: these resources won't be shared with other threads
pub(super) type ThreadOnly<T> = SyncUnsafeCell<T>;

/// task control block for a coroutine,
/// a.k.a thread in current project structure
#[repr(C, align(64))]
pub struct Task {
    // mutable
    /// task control block inner, protected by lock
    pub(super) pcb: Mutable<PCB>,
    /// user id
    pub(super) uid: AtomicU32,
    /// group id
    pub(super) gid: AtomicU32,
    /// user id - file system
    pub(super) fsuid: AtomicU32,
    /// group id - file system
    pub(super) fsgid: AtomicU32,
    /// user id - effective
    pub(super) euid: AtomicU32,
    /// group id - effective
    pub(super) egid: AtomicU32,
    /// user id - saved
    pub(super) suid: AtomicU32,
    /// group id - saved
    pub(super) sgid: AtomicU32,
    /// supplementary groups
    pub(super) sup_groups: Mutable<Vec<u32>>,

    // thread only / once initialization
    /// thread control block
    pub(super) tcb: ThreadOnly<TCB>,
    /// sched entity for the task, shared with scheduler
    pub(super) sched_entity: ThreadOnly<SchedEntity>,

    /// memory set for the task
    /// memory set can be both modified and shared
    pub(super) memory_set: ThreadOnly<SharedMut<MemorySet>>,

    // immutable
    /// task id, with lifetime holded
    pub(super) tid: Immutable<TidTracer>,
    /// task group id, aka pid
    pub(super) tgid: Immutable<TGID>,

    // shared
    /// file descriptor table
    pub(super) fd_table: SharedMut<FdTable>,
    /// current work directory
    pub(super) dir_cwd: SharedMut<Arc<dyn Dentry>>,
    /// executable file path
    pub(super) dir_exe: SharedMut<String>,
    /// root directory
    pub(super) dir_root: SharedMut<Arc<dyn Dentry>>,
    /// proc directory, used for /proc/self
    pub(super) dir_proc: SharedMut<Arc<dyn Dentry>>,
    /// signal action list, saves signal handler
    pub(super) sa_list: SharedMut<SigActionList>,
    /// thread group
    pub(super) thread_group: SharedMut<ThreadGroup>,
    /// process group id
    pub(super) pgid: Arc<AtomicUsize>,
    /// futex wait queue
    pub(super) futex: SharedMut<FutexQueue>,
    /// interval timer
    pub(super) itimer: SharedMut<ITimerManager>,
}

/// user tasks
/// - usage: wrap it in Arc<Task>
impl Task {
    /// lock the process control block
    #[inline(always)]
    pub fn pcb(&self) -> SpinLockGuard<PCB> {
        self.pcb.lock()
    }
    #[allow(dead_code)]
    pub fn try_lock_pcb(&self) -> Option<SpinLockGuard<PCB>> {
        self.pcb.try_lock()
    }

    /// tid
    #[inline(always)]
    pub fn tid(&self) -> TID {
        self.tid.0
    }
    pub fn tgid(&self) -> TGID {
        self.tgid
    }
    pub fn pid(&self) -> PID {
        self.tgid
    }
    pub fn get_pgid(&self) -> PGID {
        self.pgid.load(core::sync::atomic::Ordering::SeqCst)
    }
    pub fn set_pgid(&self, pgid: usize) {
        self.pgid.store(pgid, core::sync::atomic::Ordering::SeqCst);
    }

    /// uid & gid
    #[inline(always)]
    pub fn uid(&self) -> u32 {
        self.uid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn gid(&self) -> u32 {
        self.gid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn fsuid(&self) -> u32 {
        self.fsuid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn fsgid(&self) -> u32 {
        self.fsgid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_uid(&self, uid: u32) {
        self.uid.store(uid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn set_gid(&self, gid: u32) {
        self.gid.store(gid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn set_fsuid(&self, fsuid: u32) {
        self.fsuid
            .store(fsuid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn set_fsgid(&self, fsgid: u32) {
        self.fsgid
            .store(fsgid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn euid(&self) -> u32 {
        self.euid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn egid(&self) -> u32 {
        self.egid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_euid(&self, euid: u32) {
        self.euid.store(euid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn set_egid(&self, egid: u32) {
        self.egid.store(egid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn suid(&self) -> u32 {
        self.suid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_suid(&self, suid: u32) {
        self.suid.store(suid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn sgid(&self) -> u32 {
        self.sgid.load(core::sync::atomic::Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_sgid(&self, sgid: u32) {
        self.sgid.store(sgid, core::sync::atomic::Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn sup_groups(&self) -> SpinLockGuard<Vec<u32>> {
        self.sup_groups.lock()
    }

    /// check if the task is group leader
    /// if true, the task is also called process
    #[inline(always)]
    pub fn is_group_leader(&self) -> bool {
        self.tid() == self.tgid()
    }

    /// memory set
    #[inline(always)]
    pub fn memory_set(&self) -> &Arc<SpinLock<MemorySet>> {
        &self.memory_set.as_ref()
    }
    #[inline(always)]
    pub fn memory_activate(&self) {
        self.memory_set.as_ref().lock().memory_activate();
    }
    /// change current memory set
    pub fn change_memory_set(&self, memory_set: MemorySet) {
        *self.memory_set.as_ref_mut() = Arc::new(SpinLock::new(memory_set));
    }

    /// thread group
    pub fn thread_group(&self) -> SpinLockGuard<ThreadGroup> {
        self.thread_group.lock()
    }

    /// get fd_table
    #[inline(always)]
    pub fn fd_table(&self) -> SpinLockGuard<FdTable> {
        self.fd_table.lock()
    }
    pub fn put_fd_table(&self) {
        if Arc::strong_count(&self.fd_table) == 1 {
            // only one strong reference, we can safely drop it
            info!("[kernel] clear fd_table for task {}", self.tid());
            self.fd_table.lock().exit_files();
        }
    }

    /// get cwd
    #[inline(always)]
    pub fn cwd(&self) -> SpinLockGuard<Arc<dyn Dentry>> {
        self.dir_cwd.lock()
    }
    #[inline(always)]
    pub fn exe(&self) -> SpinLockGuard<String> {
        self.dir_exe.lock()
    }
    /// get root
    #[inline(always)]
    pub fn root(&self) -> SpinLockGuard<Arc<dyn Dentry>> {
        self.dir_root.lock()
    }

    /// trap context
    #[inline(always)]
    pub fn trap_context(&self) -> &TrapContext {
        self.tcb().cx.cx()
    }
    #[inline(always)]
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        self.tcb_mut().cx.cx_mut()
    }
    #[inline(always)]
    pub fn record_cx_int_en(&self) {
        let int_en = Arch::is_interrupt_enabled();
        self.tcb_mut().cx.int_en = int_en;
    }
    #[inline(always)]
    pub fn restore_cx_int_en(&self) {
        if self.tcb_mut().cx.int_en {
            Arch::enable_interrupt();
        } else {
            Arch::disable_interrupt();
        }
    }

    /// signal info: sigaction list
    pub fn sa_list(&self) -> SpinLockGuard<SigActionList> {
        self.sa_list.lock()
    }

    /// set waker
    pub fn set_waker(&self, waker: Waker) {
        self.tcb_mut().waker = Some(waker);
    }
    /// wake self up
    pub fn wake_unchecked(&self) {
        if let Some(waker) = self.tcb().waker.as_ref() {
            waker.wake_by_ref();
        } else {
            warn!("[kernel] waker of task {} is None", self.tid());
        }
    }

    /// tcb
    pub fn tcb(&self) -> &TCB {
        self.tcb.as_ref()
    }
    pub fn tcb_mut(&self) -> &mut TCB {
        self.tcb.as_ref_mut()
    }

    /// signal mask
    pub fn sig_mask(&self) -> SigMask {
        self.tcb().sig_mask
    }
    pub fn sig_mask_mut(&self) -> &mut SigMask {
        &mut self.tcb_mut().sig_mask
    }
    pub fn set_sig_mask(&self, mask: SigMask) {
        *self.sig_mask_mut() = mask;
    }
    pub fn set_old_mask(&self, mask: SigMask) {
        self.tcb_mut().old_mask = Some(mask);
    }
    pub fn take_old_mask(&self) -> Option<SigMask> {
        self.tcb_mut().old_mask.take()
    }
    pub fn swap_in_sigmask(&self, new_mask: SigMask) {
        self.set_old_mask(core::mem::replace(self.sig_mask_mut(), new_mask));
    }

    // tif
    pub fn tif(&self) -> &TaskFlags {
        &self.tcb().flags
    }
    pub fn tif_mut(&self) -> &mut TaskFlags {
        &mut self.tcb_mut().flags
    }

    /// sched entity
    pub fn sched_entity(&self) -> &SchedEntity {
        self.sched_entity.as_ref()
    }
    pub fn sched_entity_mut(&self) -> &mut SchedEntity {
        self.sched_entity.as_ref_mut()
    }

    /// schedule
    pub fn need_resched(&self) -> bool {
        self.sched_entity().need_yield()
            || unlikely(self.tcb().flags.contains(TaskFlags::TIF_NEED_RESCHED))
    }
    pub fn clear_resched_flags(&self) {
        self.sched_entity_mut().clear_pending_yield();
        self.tcb_mut().flags.remove(TaskFlags::TIF_NEED_RESCHED);
    }

    /// time stat
    pub fn time_stat(&self) -> &TimeInfo {
        &self.sched_entity().time_stat
    }
    pub fn time_stat_mut(&self) -> &mut TimeInfo {
        &mut self.sched_entity_mut().time_stat
    }

    /// set prio: normal
    pub fn set_sched_prio_normal(&self) {
        self.sched_entity_mut().sched_prio = SchedPrio::Normal;
    }
    /// set prio: realtime
    pub fn set_sched_prio_realtime(&self, prio: usize) {
        self.sched_entity_mut().sched_prio = SchedPrio::RealTime(prio);
    }
    /// set prio: idle
    pub fn set_sched_prio_idle(&self) {
        self.sched_entity_mut().sched_prio = SchedPrio::IdlePrio;
    }

    /// cpu mask
    pub fn cpu_mask(&self) -> &CpuMask {
        &self.sched_entity().cpu_mask
    }
    pub fn cpu_mask_mut(&self) -> &mut CpuMask {
        &mut self.sched_entity_mut().cpu_mask
    }

    /// sched entity
    pub fn get_sched_entity(&self) -> *mut SchedEntity {
        self.sched_entity.get()
    }

    /// futex wait queue
    pub fn futex(&self) -> SpinLockGuard<FutexQueue> {
        self.futex.lock()
    }

    /// user context
    pub fn ucx(&self) -> &UserPtr<UContext> {
        &self.tcb().ucx
    }
    pub fn ucx_mut(&self) -> &mut UserPtr<UContext> {
        &mut self.tcb_mut().ucx
    }

    /// interval timer manager
    pub fn itimer(&self) -> SpinLockGuard<ITimerManager> {
        self.itimer.lock()
    }

    /// fetch vfork flag
    pub fn vfork_flag(&self) -> Option<(Arc<AtomicBool>, Waker)> {
        self.tcb().vfork_wait.clone()
    }

    /// register vfork info
    /// for parent's flag fetching and child's callback
    pub fn register_vfork_info(&self, parent_waker: Waker) {
        self.tcb_mut().vfork_wait = Some((Arc::new(AtomicBool::new(false)), parent_waker));
    }
}

// process implementation
impl Task {
    #[allow(unused)]
    fn print_child_tree_dfs(&self, fmt_offset: usize) -> usize {
        let mut fmt_proc = String::new();
        for _ in 0..fmt_offset {
            fmt_proc += "|---";
        }
        let mut fmt_thread = String::new();
        for _ in 0..fmt_offset {
            fmt_thread += "|   ";
        }
        let pcb = self.pcb();
        let par_tid = pcb
            .parent
            .as_ref()
            .map(|x| x.upgrade())
            .flatten()
            .map(|x| x.tid())
            .unwrap_or(0);
        if self.is_group_leader() {
            warn!("{fmt_proc}process {}", self.tid());
        } else {
            warn!("{fmt_proc}thread {}", self.tid());
        }
        for thread in self.thread_group().0.iter() {
            let thread = thread.1.upgrade().unwrap();
            if thread.tid() == self.tid() {
                continue;
            }
            warn!("{fmt_thread}thread {}", thread.tid());
        }
        for child in &pcb.children {
            let tid = child.print_child_tree_dfs(fmt_offset + 1);
            assert!(tid == self.tid());
        }
        par_tid
    }

    /// only for debug, print current child tree
    /// warning: this function could cause deadlock if under multicore
    #[allow(unused)]
    pub fn print_child_tree(&self) {
        self.print_child_tree_dfs(0);
    }
}

//! # Task

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{
    intrinsics::{likely, unlikely},
    marker::PhantomData,
    ptr::null,
    sync::atomic::{AtomicU32, AtomicUsize},
    task::Waker,
};

use arch::{Arch, ArchInfo, ArchInt, ArchMemory, ArchTrapContext, TrapContext};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
};

use super::{
    context::TaskTrapContext,
    exit::ExitReason,
    manager::ThreadGroup,
    pcb::PCB,
    status::TaskStatus,
    taskid::{TidTracer, PGID, PID, TGID, TID},
    tcb::TCB,
};
use crate::{
    entry::init_proc::INIT_PROC_NAME,
    fs::{
        fdtable::FdTable,
        vfs::{
            basic::{dentry::Dentry, file::File},
            root_dentry,
        },
    },
    include::{
        process::{
            auxv::{AuxEntry, AT_NULL, AT_RANDOM},
            CloneFlags, TaskFlags,
        },
        sched::CpuMask,
    },
    mm::{
        memory_set::{ElfMemoryInfo, MemorySet},
        user_ptr::UserPtr,
    },
    sched::{
        sched_entity::{SchedEntity, SchedPrio},
        utils::take_waker,
    },
    signal::{
        sig_action::SigActionList,
        sig_manager::SigManager,
        sig_set::{SigMask, SigSet},
        sig_stack::UContext,
    },
    syscall::SysResult,
    task::{
        futex::FutexQueue,
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        taskid::tid_alloc,
    },
    time::{time_info::TimeInfo, timer::ITimerManager},
};

/// shared between threads
type SharedMut<T> = Arc<SpinLock<T>>;
struct Shared<T>(PhantomData<T>);
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
type Mutable<T> = SpinLock<T>;

/// read-only resources, could be shared safely through threads
type Immutable<T> = T;

/// only used in current thread, mutable resources without lock
/// SAFETY: these resources won't be shared with other threads
type ThreadOnly<T> = SyncUnsafeCell<T>;

/// task control block for a coroutine,
/// a.k.a thread in current project structure
#[repr(C, align(64))]
pub struct Task {
    // mutable
    pcb: Mutable<PCB>,             // task control block inner, protected by lock
    uid: AtomicU32,                // user id
    gid: AtomicU32,                // group id
    fsuid: AtomicU32,              // user id - file system
    fsgid: AtomicU32,              // group id - file system
    euid: AtomicU32,               // user id - effective
    egid: AtomicU32,               // group id - effective
    suid: AtomicU32,               // user id - saved
    sgid: AtomicU32,               // group id - saved
    sup_groups: Mutable<Vec<u32>>, // supplementary groups

    // thread only / once initialization
    tcb: ThreadOnly<TCB>,                  // thread control block
    sched_entity: ThreadOnly<SchedEntity>, // sched entity for the task, shared with scheduler

    // immutable
    tid: Immutable<TidTracer>, // task id, with lifetime holded
    tgid: Immutable<TGID>,     // task group id, aka pid

    // shared
    fd_table: SharedMut<FdTable>,         // file descriptor table
    cwd: SharedMut<Arc<dyn Dentry>>,      // current work directory
    exe: SharedMut<String>,               // executable file path
    root: SharedMut<Arc<dyn Dentry>>,     // root directory
    sa_list: SharedMut<SigActionList>,    // signal action list, saves signal handler
    memory_set: SharedMut<MemorySet>,     // memory set for the task
    thread_group: SharedMut<ThreadGroup>, // thread group
    pgid: Arc<AtomicUsize>,               // process group id
    futex: SharedMut<FutexQueue>,         // futex wait queue
    itimer: SharedMut<ITimerManager>,     // interval timer
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
        &self.memory_set
    }
    #[inline(always)]
    pub fn memory_activate(&self) {
        self.memory_set.lock().memory_activate();
    }
    /// change current memory set
    pub fn change_memory_set(&self, memory_set: MemorySet) {
        let mut ms = self.memory_set().lock();
        *ms = memory_set;
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
        self.cwd.lock()
    }
    #[inline(always)]
    pub fn exe(&self) -> SpinLockGuard<String> {
        self.exe.lock()
    }
    /// get root
    #[inline(always)]
    pub fn root(&self) -> SpinLockGuard<Arc<dyn Dentry>> {
        self.root.lock()
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
}

// process implementation
impl Task {
    pub fn try_get_status(&self) -> Option<TaskStatus> {
        if likely(!self.tif().contains(TaskFlags::TIF_STATUS_CHANGED)) {
            None
        } else {
            self.tif_mut().remove(TaskFlags::TIF_STATUS_CHANGED);
            Some(self.pcb().status())
        }
    }

    /// exit current task
    pub fn terminate(&self, exit_code: ExitReason) {
        let mut pcb = self.pcb();
        if self.is_group_leader() {
            pcb.set_exit_code(exit_code);
        }
        pcb.set_status(TaskStatus::Terminated, self.tif_mut());
    }

    /// terminate all tasks in current thread group
    pub fn terminate_group(&self, exit_code: ExitReason) {
        let tg = self.thread_group();
        for (_id, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            task.terminate(exit_code);
        }
    }

    /// terminate all tasks except group leader in current thread group
    pub fn terminate_threads(&self) {
        assert!(self.is_group_leader());
        let tg = self.thread_group();
        for (_id, t) in tg.0.iter() {
            let task = t.upgrade().unwrap();
            if !task.is_group_leader() {
                task.terminate(ExitReason::default());
            }
        }
    }

    /// create new init process from elf
    pub async fn new_init_process(elf: ElfMemoryInfo) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            entry_point: elf_entry,
            user_sp,
            auxs: _,
        } = elf;
        let user_sp = user_sp - 16;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid = tid_alloc();
        let tgid = tid.0;
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            pgid: Shared::new_atomic(tgid),
            pcb: Mutable::new(PCB {
                signals: SigManager::default(),
                ..Default::default()
            }),
            thread_group: Shared::new(ThreadGroup::new()),
            memory_set: Shared::new(memory_set),
            sched_entity: ThreadOnly::new(SchedEntity::default()),
            fd_table: Shared::new(FdTable::new()),
            cwd: Shared::new(root_dentry()),
            exe: Shared::new(format!("/{}", INIT_PROC_NAME)), // executable path
            root: Shared::new(root_dentry()),
            sa_list: Shared::new(SigActionList::new()),
            tcb: ThreadOnly::new(TCB {
                cx: TaskTrapContext::new(TrapContext::app_init_cx(elf_entry, user_sp), true),
                sig_mask: SigSet::all(),
                ..Default::default()
            }),
            futex: Shared::new(FutexQueue::new()),
            itimer: Shared::new(ITimerManager::new()),
            uid: AtomicU32::new(0),               // default user id
            gid: AtomicU32::new(0),               // default group id
            fsuid: AtomicU32::new(0),             // default fs user id
            fsgid: AtomicU32::new(0),             // default fs group id
            euid: AtomicU32::new(0),              // default effective user id
            egid: AtomicU32::new(0),              // default effective group id
            suid: AtomicU32::new(0),              // default saved user id
            sgid: AtomicU32::new(0),              // default saved group id
            sup_groups: Mutable::new(Vec::new()), // default supplementary groups
        });
        task.thread_group().insert(&task);
        TASK_MANAGER.insert(&task);
        PROCESS_GROUP_MANAGER.lock().insert(&task);
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// init user stack with pushing arg, env, and auxv
    pub fn init_user_stack(
        &self,
        mut user_sp: usize,
        args: Vec<String>,        // argv & argc
        envs: Vec<String>,        // env vec
        auxs: &mut Vec<AuxEntry>, // aux vec
    ) -> (usize, usize, usize, usize) {
        /// push a data slice with alignment
        /// this func will update user_sp
        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp
        }
        /// align sp with 16 bytes (usize*2)
        macro_rules! align_16 {
            ($sp:ident) => {
                $sp = $sp & !0xf;
            };
        }

        // argv, envp are vectors of each arg's/env's addr
        let mut argv = vec![0; args.len()];
        let mut envp = vec![0; envs.len()];

        // copy each env to the newly allocated stack
        for i in 0..envs.len() {
            // here we leave one byte to store a '\0' as a terminator
            user_sp -= envs[i].len() + 1;
            let p: *mut u8 = user_sp as *mut u8;
            unsafe {
                envp[i] = user_sp;
                p.copy_from(envs[i].as_ptr(), envs[i].len());
                *((p as usize + envs[i].len()) as *mut u8) = 0;
            }
        }
        align_16!(user_sp);

        // copy each arg to the newly allocated stack
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            let p = user_sp as *mut u8;
            unsafe {
                argv[i] = user_sp;
                p.copy_from(args[i].as_ptr(), args[i].len());
                *((p as usize + args[i].len()) as *mut u8) = 0;
            }
        }
        align_16!(user_sp);

        // copy platform
        let platform = Arch::ARCH_NAME;
        user_sp -= platform.len() + 1;
        align_16!(user_sp);
        let p = user_sp as *mut u8;
        unsafe {
            p.copy_from(platform.as_ptr(), platform.len());
            *((p as usize + platform.len()) as *mut u8) = 0;
        }

        // copy 16 random bytes (here is 0)
        user_sp -= 16;
        auxs.push(AuxEntry(AT_RANDOM, user_sp as usize));
        auxs.push(AuxEntry(AT_NULL, 0 as usize));
        align_16!(user_sp);

        // construct auxv
        let len = auxs.len() * core::mem::size_of::<AuxEntry>();
        user_sp -= len;
        for i in 0..auxs.len() {
            unsafe {
                *((user_sp + i * core::mem::size_of::<AuxEntry>()) as *mut usize) = auxs[i].0;
                *((user_sp + i * core::mem::size_of::<AuxEntry>() + core::mem::size_of::<usize>())
                    as *mut usize) = auxs[i].1;
            }
        }

        // construct envp
        let len = (envs.len() + 1) * core::mem::size_of::<usize>();
        user_sp -= len;
        let envp_base = user_sp;
        for i in 0..envs.len() {
            unsafe { *((envp_base + i * core::mem::size_of::<usize>()) as *mut usize) = envp[i] };
        }
        unsafe { *((envp_base + envs.len() * core::mem::size_of::<usize>()) as *mut usize) = 0 };

        // push argv, argc
        push_slice(&mut user_sp, &[null::<u8>()]);
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;
        push_slice(&mut user_sp, &[args.len()]);
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            let new_memory_set = self.memory_set().lock().clone_cow();
            Arch::tlb_flush();
            Shared::new(new_memory_set)
        };

        let sa_list = if flags.contains(CloneFlags::SIGHAND) {
            self.sa_list.clone()
        } else {
            Shared::new(self.sa_list.lock().clone())
        };

        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            Shared::new(self.fd_table.lock().clone())
        };

        // CLONE_PARENT (since Linux 2.3.12)
        //   If CLONE_PARENT is set, then the parent of the new child
        //   (as returned by getppid(2)) will be the same as that of the
        //   calling process.
        // If CLONE_PARENT is not set, then (as with fork(2)) the
        // child's parent is the calling process.
        let parent = if flags.contains(CloneFlags::PARENT) {
            self.pcb.lock().parent.clone()
        } else {
            Some(Arc::downgrade(self))
        };

        let sup_groups = Mutable::new(self.sup_groups().clone());

        let res = if flags.contains(CloneFlags::THREAD) {
            // fork as a new thread
            let new_tid = tid_alloc();
            let tid_val = new_tid.0;
            info!("fork new thread, tid: {}", tid_val);
            let new_thread = Arc::new(Self {
                tid: new_tid,
                tgid: self.tgid.clone(),
                pgid: self.pgid.clone(),
                thread_group: self.thread_group.clone(),
                pcb: Mutable::new(PCB {
                    parent,
                    ..Default::default()
                }),
                memory_set,
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                cwd: self.cwd.clone(),
                exe: self.exe.clone(),
                root: self.root.clone(),
                sa_list,
                tcb: ThreadOnly::new(TCB {
                    cx: TaskTrapContext::new(self.trap_context().clone(), true),
                    ..Default::default()
                }),
                futex: self.futex.clone(),
                itimer: self.itimer.clone(),
                uid: AtomicU32::new(self.uid()),
                gid: AtomicU32::new(self.gid()),
                fsuid: AtomicU32::new(self.fsuid()),
                fsgid: AtomicU32::new(self.fsgid()),
                euid: AtomicU32::new(self.euid()),
                egid: AtomicU32::new(self.egid()),
                suid: AtomicU32::new(self.suid()),
                sgid: AtomicU32::new(self.sgid()),
                sup_groups,
            });
            new_thread.thread_group.lock().insert(&new_thread);
            TASK_MANAGER.insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            let new_tgid = new_tid.0;
            let new_pgid = self.get_pgid(); // use parent's pgid
            info!("fork new process, tgid: {}", new_tgid);
            let new_process = Arc::new(Self {
                tid: new_tid,
                tgid: new_tgid,
                pgid: Shared::new_atomic(new_pgid),
                thread_group: Shared::new(ThreadGroup::new()),
                pcb: Mutable::new(PCB {
                    parent,
                    ..Default::default()
                }),
                memory_set,
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                cwd: Shared::new(self.cwd().clone()),
                exe: Shared::new(self.exe().clone()),
                root: Shared::new(self.root().clone()),
                sa_list,
                tcb: ThreadOnly::new(TCB {
                    cx: TaskTrapContext::new(self.trap_context().clone(), true),
                    ..Default::default()
                }),
                futex: Shared::new(FutexQueue::new()),
                itimer: Shared::new(ITimerManager::new()),
                uid: AtomicU32::new(self.uid()),
                gid: AtomicU32::new(self.gid()),
                fsuid: AtomicU32::new(self.fsuid()),
                fsgid: AtomicU32::new(self.fsgid()),
                euid: AtomicU32::new(self.euid()),
                egid: AtomicU32::new(self.egid()),
                suid: AtomicU32::new(self.suid()),
                sgid: AtomicU32::new(self.sgid()),
                sup_groups,
            });
            new_process.thread_group().insert(&new_process);
            self.pcb().children.push(new_process.clone());
            TASK_MANAGER.insert(&new_process);
            PROCESS_GROUP_MANAGER.lock().insert(&new_process);
            new_process
        };
        res
    }

    /// execute
    pub async fn execve(
        self: &Arc<Self>,
        elf_file: Arc<dyn File>,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> SysResult<()> {
        let ElfMemoryInfo {
            memory_set,
            entry_point,
            user_sp,
            mut auxs,
        } = MemorySet::load_elf(&elf_file).await?;
        memory_set.memory_activate();
        *self.exe() = elf_file.path();
        self.terminate_threads();
        self.change_memory_set(memory_set);
        let (user_sp, _argc, _argv_base, _envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        *self.trap_context_mut() = TrapContext::app_init_cx(entry_point, user_sp);
        self.sa_list().reset();
        self.fd_table().close_on_exec();
        Ok(())
    }

    /// init thread only resources
    pub async fn thread_init(self: &Arc<Self>) {
        if let Some(tid) = self.tcb().set_child_tid {
            let ptr = UserPtr::<usize>::new(tid);
            let _ = ptr.write(self.tid()).await.inspect_err(|err| {
                error!(
                    "[kernel] failed to write set_child_tid: {}, tid: {}",
                    err,
                    self.tid()
                )
            });
        }
        self.set_waker(take_waker().await);
    }

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

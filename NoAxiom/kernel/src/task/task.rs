//! # Task

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{marker::PhantomData, ptr::null, sync::atomic::AtomicUsize, task::Waker};

use arch::{Arch, ArchInfo, ArchInt, ArchMemory, ArchTrapContext, TrapContext};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
    Once,
};

use super::{
    context::TaskContext,
    exit::ExitCode,
    manager::ThreadGroup,
    status::TaskStatus,
    taskid::{TidTracer, PGID, PID, TGID, TID},
};
use crate::{
    fs::{fdtable::FdTable, path::Path},
    include::{
        fs::InodeMode,
        process::{
            auxv::{AuxEntry, AT_NULL, AT_RANDOM},
            robust_list::RobustList,
            CloneFlags,
        },
        sched::CpuMask,
        syscall_id::SyscallID,
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
        sig_pending::SigPending,
        sig_set::{SigMask, SigSet},
        sig_stack::{SigAltStack, UContext},
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

/// task control block inner
/// it is protected by a spinlock to assure its atomicity
/// so there's no need to use any lock in this struct
#[repr(align(64))]
pub struct PCB {
    // task status
    pub status: TaskStatus,  // task status
    pub exit_code: ExitCode, // exit code

    // paternity
    // assertion: only when the task is group leader, it can have children
    pub children: Vec<Arc<Task>>,   // children tasks
    pub parent: Option<Weak<Task>>, // parent task, weak ptr

    // signal structs
    pub pending_sigs: SigPending,       // pending signals
    pub sig_stack: Option<SigAltStack>, // signal alternate stack

    // futex & robust list
    pub robust_list: RobustList,
}

impl Default for PCB {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            parent: None,
            status: TaskStatus::Normal,
            exit_code: ExitCode::default(),
            pending_sigs: SigPending::new(),
            sig_stack: None,
            robust_list: RobustList::default(),
        }
    }
}

pub struct TCB {
    pub set_child_tid: Option<usize>,   // set tid address
    pub clear_child_tid: Option<usize>, // clear tid address
    pub current_syscall: SyscallID,     // only for debug, current syscall id
}

impl Default for TCB {
    fn default() -> Self {
        Self {
            set_child_tid: None,
            clear_child_tid: None,
            current_syscall: SyscallID::NO_SYSCALL,
        }
    }
}

/// task control block for a coroutine,
/// a.k.a thread in current project structure
#[repr(C, align(64))]
pub struct Task {
    // mutable
    pcb: Mutable<PCB>, // task control block inner, protected by lock

    // thread only / once initialization
    tcb: ThreadOnly<TCB>,                  // thread control block
    cx: ThreadOnly<TaskContext>,           // trap context
    sched_entity: ThreadOnly<SchedEntity>, // sched entity for the task
    waker: Once<Waker>,                    // waker for the task
    ucx: ThreadOnly<UserPtr<UContext>>,    // ucontext for the task

    // immutable
    tid: Immutable<TidTracer>,              // task id, with lifetime holded
    tgid: Immutable<TGID>,                  // task group id, aka pid
    tg_leader: Immutable<Once<Weak<Task>>>, // thread group leader

    // shared
    fd_table: SharedMut<FdTable>,         // file descriptor table
    cwd: SharedMut<Path>,                 // current work directory
    sa_list: SharedMut<SigActionList>,    // signal action list, saves signal handler
    memory_set: SharedMut<MemorySet>,     // memory set for the task
    thread_group: SharedMut<ThreadGroup>, // thread group
    pgid: Arc<AtomicUsize>,               // process group id
    futex: SharedMut<FutexQueue>,         // futex wait queue
    itimer: SharedMut<ITimerManager>,     // interval timer
}

impl PCB {
    // task status
    #[inline(always)]
    pub fn status(&self) -> TaskStatus {
        self.status
    }
    #[inline(always)]
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    // exit code
    pub fn exit_code(&self) -> ExitCode {
        self.exit_code
    }
    pub fn set_exit_code(&mut self, exit_code: ExitCode) {
        self.exit_code = exit_code;
    }

    /// set wake signal
    pub fn set_wake_signal(&mut self, sig: SigSet) {
        self.pending_sigs.should_wake = sig;
    }
    /// signal mask
    pub fn sig_mask(&self) -> SigMask {
        self.pending_sigs.sig_mask
    }
    pub fn sig_mask_mut(&mut self) -> &mut SigMask {
        &mut self.pending_sigs.sig_mask
    }

    /// find zombie children
    pub fn pop_one_zombie_child(&mut self) -> Option<Arc<Task>> {
        let mut res = None;
        for i in 0..self.children.len() {
            if self.children[i].pcb().status() == TaskStatus::Zombie {
                res = Some(self.children.remove(i));
                break;
            }
        }
        res
    }
}

/// user tasks
/// - usage: wrap it in Arc<Task>
impl Task {
    /// lock the process control block
    #[inline(always)]
    pub fn pcb(&self) -> SpinLockGuard<PCB> {
        self.pcb.lock()
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
    pub fn set_self_as_tg_leader(self: &Arc<Self>) {
        self.tg_leader.call_once(|| Arc::downgrade(self));
    }
    pub fn set_tg_leader_weakly(&self, task: &Weak<Self>) {
        self.tg_leader.call_once(|| task.clone());
    }
    pub fn get_tg_leader(&self) -> Arc<Task> {
        self.tg_leader.get().unwrap().upgrade().unwrap()
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

    /// get cwd
    #[inline(always)]
    pub fn cwd(&self) -> SpinLockGuard<Path> {
        self.cwd.lock()
    }

    /// trap context
    #[inline(always)]
    pub fn trap_context(&self) -> &TrapContext {
        self.cx.as_ref().cx()
    }
    #[inline(always)]
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        self.cx.as_ref().cx_mut()
    }
    #[inline(always)]
    pub fn record_cx_int_en(&self) {
        let int_en = Arch::is_interrupt_enabled();
        self.cx.as_ref_mut().int_en = int_en;
    }
    #[inline(always)]
    pub fn restore_cx_int_en(&self) {
        if self.cx.as_ref().int_en {
            Arch::enable_interrupt();
        } else {
            Arch::disable_interrupt();
        }
    }
    #[inline(always)]
    pub fn cx_int_en(&self) -> bool {
        self.cx.as_ref().int_en
    }

    /// signal info: sigaction list
    pub fn sa_list(&self) -> SpinLockGuard<SigActionList> {
        self.sa_list.lock()
    }

    /// get waker
    pub fn waker(&self) -> Option<Waker> {
        self.waker.get().cloned()
    }
    /// set waker
    pub fn set_waker(&self, waker: Waker) {
        self.waker.call_once(|| waker);
    }
    /// wake self up
    pub fn wake_unchecked(&self) {
        if let Some(waker) = self.waker.get() {
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

    /// sched entity
    pub fn sched_entity(&self) -> &SchedEntity {
        self.sched_entity.as_ref()
    }
    pub fn sched_entity_mut(&self) -> &mut SchedEntity {
        self.sched_entity.as_ref_mut()
    }

    /// time stat
    pub fn time_stat(&self) -> &TimeInfo {
        &self.sched_entity.as_ref_mut().time_stat
    }
    pub fn time_stat_mut(&self) -> &mut TimeInfo {
        &mut self.sched_entity.as_ref_mut().time_stat
    }

    /// set prio: normal
    pub fn set_sched_prio_normal(&self) {
        self.sched_entity.as_ref_mut().sched_prio = SchedPrio::Normal;
    }
    /// set prio: realtime
    pub fn set_sched_prio_realtime(&self, prio: usize) {
        self.sched_entity.as_ref_mut().sched_prio = SchedPrio::RealTime(prio);
    }
    /// set prio: idle
    pub fn set_sched_prio_idle(&self) {
        self.sched_entity.as_ref_mut().sched_prio = SchedPrio::IdlePrio;
    }

    /// cpu mask
    pub fn cpu_mask(&self) -> &CpuMask {
        &self.sched_entity.as_ref_mut().cpu_mask
    }
    pub fn cpu_mask_mut(&self) -> &mut CpuMask {
        &mut self.sched_entity.as_ref_mut().cpu_mask
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
        self.ucx.as_ref()
    }
    pub fn ucx_mut(&self) -> &mut UserPtr<UContext> {
        self.ucx.as_ref_mut()
    }

    /// interval timer manager
    pub fn itimer(&self) -> SpinLockGuard<ITimerManager> {
        self.itimer.lock()
    }
}

// process implementation
impl Task {
    /// exit current task
    pub fn terminate(&self, exit_code: ExitCode) {
        let mut pcb = self.pcb();
        if self.is_group_leader() {
            pcb.set_exit_code(exit_code);
        }
        pcb.set_status(TaskStatus::Terminated);
    }

    /// terminate all tasks in current thread group
    pub fn terminate_group(&self, exit_code: ExitCode) {
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
                task.terminate(ExitCode::default());
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
        // def root path
        let path = Path::from_or_create(format!("/"), InodeMode::DIR)
            .await
            .unwrap();
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            pgid: Shared::new_atomic(tgid),
            pcb: Mutable::new(PCB {
                pending_sigs: SigPending {
                    sig_mask: SigSet::all() - SigSet::SIGCHLD,
                    ..Default::default()
                },
                ..Default::default()
            }),
            thread_group: Shared::new(ThreadGroup::new()),
            memory_set: Shared::new(memory_set),
            cx: ThreadOnly::new(TaskContext::new(
                TrapContext::app_init_cx(elf_entry, user_sp),
                true,
            )),
            sched_entity: ThreadOnly::new(SchedEntity::default()),
            fd_table: Shared::new(FdTable::new()),
            cwd: Shared::new(path),
            sa_list: Shared::new(SigActionList::new()),
            waker: Once::new(),
            tg_leader: Once::new(),
            tcb: ThreadOnly::new(TCB {
                ..Default::default()
            }),
            futex: Shared::new(FutexQueue::new()),
            ucx: ThreadOnly::new(UserPtr::new_null()),
            itimer: Shared::new(ITimerManager::new()),
        });
        task.thread_group().insert(&task);
        task.set_self_as_tg_leader();
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
                cx: ThreadOnly::new(TaskContext::new(self.trap_context().clone(), true)),
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                cwd: self.cwd.clone(),
                sa_list,
                waker: Once::new(),
                tg_leader: Once::new(),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
                futex: self.futex.clone(),
                ucx: ThreadOnly::new(UserPtr::new_null()),
                itimer: self.itimer.clone(),
            });
            new_thread.set_tg_leader_weakly(self.tg_leader.get().unwrap());
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
                cx: ThreadOnly::new(TaskContext::new(self.trap_context().clone(), true)),
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                cwd: Shared::new(self.cwd().clone()),
                sa_list,
                waker: Once::new(),
                tg_leader: Once::new(),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
                futex: Shared::new(FutexQueue::new()),
                ucx: ThreadOnly::new(UserPtr::new_null()),
                itimer: Shared::new(ITimerManager::new()),
            });
            new_process.thread_group().insert(&new_process);
            new_process.set_self_as_tg_leader();
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
        path: Path,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> SysResult<()> {
        let elf_file = path.dentry().open()?;
        let ElfMemoryInfo {
            memory_set,
            entry_point,
            user_sp,
            mut auxs,
        } = MemorySet::load_elf(&elf_file).await?;
        memory_set.memory_activate();
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
    fn print_child_tree_dfs(&self, fmt_offset: usize) {
        let mut fmt_proc = String::new();
        for _ in 0..fmt_offset {
            fmt_proc += "|---";
        }
        let mut fmt_thread = String::new();
        for _ in 0..fmt_offset {
            fmt_thread += "|   ";
        }
        let pcb = self.pcb();
        debug!("{fmt_proc}process {}", self.tid());
        for thread in self.thread_group().0.iter() {
            let thread = thread.1.upgrade().unwrap();
            debug!("{fmt_thread}thread {}", thread.tid());
        }
        for child in &pcb.children {
            child.print_child_tree_dfs(fmt_offset + 1);
        }
    }

    /// only for debug, print current child tree
    /// warning: this function could cause deadlock if under multicore
    #[allow(unused)]
    pub fn print_child_tree(&self) {
        self.print_child_tree_dfs(0);
    }
}

//! # Task

use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    marker::PhantomData,
    ptr::{self, null},
    task::Waker,
};

use arch::{Arch, ArchMemory, ArchTrapContext, TrapContext};
use config::fs::ROOT_NAME;
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
    Once,
};

use super::{
    exit::ExitCode,
    manager::ThreadGroup,
    status::TaskStatus,
    taskid::{TidTracer, PGID, TGID, TID},
};
use crate::{
    config::task::INIT_PROCESS_ID,
    constant::fs::{STD_ERR, STD_IN, STD_OUT},
    fs::{
        fdtable::{FdTable, FdTableEntry},
        path::Path,
    },
    include::{
        fs::InodeMode,
        process::{
            auxv::{AuxEntry, AT_NULL, AT_RANDOM},
            robust_list::RobustList,
        },
        sched::CloneFlags,
    },
    mm::{
        memory_set::{ElfMemoryInfo, MemorySet},
        user_ptr::UserPtr,
    },
    sched::sched_entity::SchedEntity,
    signal::{
        sig_action::SigActionList,
        sig_pending::SigPending,
        sig_set::{SigMask, SigSet},
        sig_stack::{SigAltStack, UContext},
    },
    syscall::SysResult,
    task::{
        manager::{PROCESS_GROUP_MANAGER, TASK_MANAGER},
        taskid::tid_alloc,
    },
    time::time_info::TimeInfo,
};

/// shared between threads
type SharedMut<T> = Arc<SpinLock<T>>;
struct Shared<T>(PhantomData<T>);
impl<T> Shared<T> {
    pub fn new(data: T) -> SharedMut<T> {
        Arc::new(SpinLock::new(data))
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
    pub children: Vec<Arc<Task>>,        // children tasks
    pub zombie_children: Vec<Arc<Task>>, // zombie children
    pub parent: Option<Weak<Task>>,      // parent task, weak ptr

    // signal structs
    pub pending_sigs: SigPending,        // pending signals
    pub sig_stack: Option<SigAltStack>,  // signal alternate stack
    pub ucontext_ptr: UserPtr<UContext>, // ucontext pointer

    // futex & robust list
    pub robust_list: RobustList,
}

impl Default for PCB {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            zombie_children: Vec::new(),
            parent: None,
            status: TaskStatus::Runnable,
            exit_code: ExitCode::default(),
            pending_sigs: SigPending::new(),
            sig_stack: None,
            ucontext_ptr: UserPtr::new_null(),
            robust_list: RobustList::default(),
        }
    }
}

pub struct TCB {
    pub clear_child_tid: Option<usize>, // clear tid address
    pub time_stat: TimeInfo,            // task time
}

impl Default for TCB {
    fn default() -> Self {
        Self {
            clear_child_tid: None,
            time_stat: TimeInfo::default(),
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
    tcb: ThreadOnly<TCB>,             // thread control block
    trap_cx: ThreadOnly<TrapContext>, // trap context
    sched_entity: SchedEntity,        // sched entity, shared with scheduler
    waker: Once<Waker>,               // waker for the task

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
    pgid: SharedMut<PGID>,                // process group id
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
    #[inline(always)]
    pub fn set_suspend(&mut self) {
        self.set_status(TaskStatus::Suspend);
    }
    #[inline(always)]
    pub fn set_runnable(&mut self) {
        self.set_status(TaskStatus::Runnable);
    }
    #[inline(always)]
    pub fn is_suspend(&self) -> bool {
        self.status() == TaskStatus::Suspend
    }

    // exit code
    pub fn exit_code(&self) -> i32 {
        self.exit_code.inner()
    }
    pub fn set_exit_code(&mut self, exit_code: ExitCode) {
        self.exit_code = exit_code;
    }

    /// suspend task can be woken up
    pub fn can_wake(&self) -> bool {
        self.is_suspend()
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
    pub fn pgid(&self) -> SpinLockGuard<PGID> {
        self.pgid.lock()
    }
    pub fn get_pgid(&self) -> PGID {
        *self.pgid.lock()
    }
    pub fn set_pgid(&self, pgid: usize) {
        *self.pgid.lock() = pgid;
        // self.pgid = SharedMutable::new(pgid);
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
        self.trap_cx.as_ref()
    }
    #[inline(always)]
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        self.trap_cx.as_ref_mut()
    }

    /// signal info: sigaction list
    pub fn sa_list(&self) -> SpinLockGuard<SigActionList> {
        self.sa_list.lock()
    }

    /// get waker
    pub fn waker(&self) -> Waker {
        self.waker.get().unwrap().clone()
    }
    /// set waker
    pub fn set_waker(&self, waker: Waker) {
        self.waker.call_once(|| waker);
    }
    /// wake self up
    pub fn wake_unchecked(&self) {
        self.waker.get().unwrap().wake_by_ref();
    }

    /// tcb
    pub fn tcb(&self) -> &TCB {
        self.tcb.as_ref()
    }
    pub fn tcb_mut(&self) -> &mut TCB {
        self.tcb.as_ref_mut()
    }

    /// sched entity
    pub fn sched_entity_ref_cloned(&self) -> SchedEntity {
        self.sched_entity.ref_clone(self.tid())
    }
    pub fn sched_entity(&self) -> &SchedEntity {
        &self.sched_entity
    }

    /// clear child tid address
    pub fn clear_child_tid(&self) -> Option<usize> {
        self.tcb().clear_child_tid
    }
    pub fn set_clear_tid_address(&self, value: usize) {
        self.tcb_mut().clear_child_tid = Some(value)
    }

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
}

// process implementation
impl Task {
    /// create new process from elf
    pub async fn new_process(elf: ElfMemoryInfo) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            entry_point: elf_entry,
            user_sp,
            auxs: _,
        } = elf;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid = tid_alloc();
        let tgid = tid.0;
        // def root path
        let path = Path::from_or_create(ROOT_NAME.to_string(), InodeMode::DIR).await;
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            pgid: Shared::new(tgid),
            pcb: Mutable::new(PCB::default()),
            thread_group: Shared::new(ThreadGroup::new()),
            memory_set: Shared::new(memory_set),
            trap_cx: ThreadOnly::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            sched_entity: SchedEntity::new_bare(INIT_PROCESS_ID),
            fd_table: Shared::new(FdTable::new()),
            cwd: Shared::new(path),
            sa_list: Shared::new(SigActionList::new()),
            waker: Once::new(),
            tg_leader: Once::new(),
            tcb: ThreadOnly::new(TCB {
                ..Default::default()
            }),
        });
        task.thread_group().insert(&task);
        task.tg_leader.call_once(|| Arc::downgrade(&task));
        TASK_MANAGER.insert(&task);
        PROCESS_GROUP_MANAGER.insert_new_group(&task);
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// init user stack with pushing arg, env, and auxv
    pub fn init_user_stack(
        &self,
        user_sp: usize,
        args: Vec<String>,        // argv & argc
        envs: Vec<String>,        // env vec
        auxs: &mut Vec<AuxEntry>, // aux vec
    ) -> (usize, usize, usize, usize) {
        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp
        }
        let mut user_sp = user_sp;

        // argv is a vector of each arg's addr
        let mut argv = vec![0; args.len()];
        // envp is a vector of each env's addr
        let mut envp = vec![0; envs.len()];
        // Copy each env to the newly allocated stack
        for i in 0..envs.len() {
            // Here we leave one byte to store a '\0' as a terminator
            user_sp -= envs[i].len() + 1;
            let p: *mut u8 = user_sp as *mut u8;
            unsafe {
                envp[i] = user_sp;
                p.copy_from(envs[i].as_ptr(), envs[i].len());
                *((p as usize + envs[i].len()) as *mut u8) = 0;
            }
        }
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // Copy each arg to the newly allocated stack
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            let p = user_sp as *mut u8;
            unsafe {
                argv[i] = user_sp;
                p.copy_from(args[i].as_ptr(), args[i].len());
                *((p as usize + args[i].len()) as *mut u8) = 0;
            }
        }
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // Copy `platform`
        let platform = "RISC-V64";
        user_sp -= platform.len() + 1;
        user_sp -= user_sp % core::mem::size_of::<usize>();
        let p = user_sp as *mut u8;
        unsafe {
            p.copy_from(platform.as_ptr(), platform.len());
            *((p as usize + platform.len()) as *mut u8) = 0;
        }

        // Copy 16 random bytes(here is 0)
        user_sp -= 16;
        auxs.push(AuxEntry(AT_RANDOM, user_sp as usize));
        // Padding
        user_sp -= user_sp % 16;
        auxs.push(AuxEntry(AT_NULL, 0 as usize));

        // Construct auxv
        let len = auxs.len() * core::mem::size_of::<AuxEntry>();
        user_sp -= len;
        for i in 0..auxs.len() {
            unsafe {
                *((user_sp + i * core::mem::size_of::<AuxEntry>()) as *mut usize) = auxs[i].0;
                *((user_sp + i * core::mem::size_of::<AuxEntry>() + core::mem::size_of::<usize>())
                    as *mut usize) = auxs[i].1;
            }
        }

        // Construct envp
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
            let (ms, _) = self.memory_set().lock().clone_cow();
            Arch::tlb_flush();
            Shared::new(ms)
        };

        let sa_list = if flags.contains(CloneFlags::SIGHAND) {
            self.sa_list.clone()
        } else {
            Shared::new(self.sa_list.lock().clone())
        };

        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            trace!("fd table info cloned");
            let tmp = Shared::new(self.fd_table.lock().clone());
            let mut guard = tmp.lock();
            guard.table[STD_IN] = Some(FdTableEntry::std_in());
            guard.table[STD_OUT] = Some(FdTableEntry::std_out());
            guard.table[STD_ERR] = Some(FdTableEntry::std_err());
            drop(guard);
            tmp
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
                    parent: self.pcb.lock().parent.clone(),
                    ..Default::default()
                }),
                memory_set,
                trap_cx: ThreadOnly::new(self.trap_context().clone()),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                cwd: self.cwd.clone(),
                sa_list,
                waker: Once::new(),
                tg_leader: Once::new(),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
            });
            new_thread.set_tg_leader_weakly(self.tg_leader.get().unwrap());
            new_thread.thread_group.lock().insert(&new_thread);
            TASK_MANAGER.insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            let new_tgid = new_tid.0;
            let new_pgid = *self.pgid(); // use parent's pgid
            info!("fork new process, tgid: {}", new_tgid);
            let new_process = Arc::new(Self {
                tid: new_tid,
                tgid: new_tgid,
                pgid: Shared::new(new_pgid),
                thread_group: Shared::new(ThreadGroup::new()),
                pcb: Mutable::new(PCB {
                    parent: Some(Arc::downgrade(self)),
                    ..Default::default()
                }),
                memory_set,
                trap_cx: ThreadOnly::new(self.trap_context().clone()),
                sched_entity: self.sched_entity.data_clone(new_tgid),
                fd_table,
                cwd: Shared::new(self.cwd().clone()),
                sa_list,
                waker: Once::new(),
                tg_leader: Once::new(),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
            });
            new_process.thread_group().insert(&new_process);
            new_process.set_self_as_tg_leader();
            self.pcb().children.push(new_process.clone()); // fixme: might use tg leader
            TASK_MANAGER.insert(&new_process);
            PROCESS_GROUP_MANAGER.insert_process(new_pgid, &new_process);
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
        let ElfMemoryInfo {
            memory_set,
            entry_point,
            user_sp,
            mut auxs,
        } = MemorySet::load_from_path(path).await?;
        memory_set.memory_activate();
        self.terminate_threads();
        self.change_memory_set(memory_set);
        trace!("init usatck");
        let (user_sp, argc, argv_base, envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        self.trap_context_mut()
            .update_cx(entry_point, user_sp, argc, argv_base, envp_base);
        self.sa_list().reset();
        self.fd_table().close_on_exec();
        Ok(())
    }
}

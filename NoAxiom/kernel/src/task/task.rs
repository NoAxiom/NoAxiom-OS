//! # Task

use alloc::{
    borrow::ToOwned,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::task::Waker;

use arch::{ArchTrapContext, TrapArgs, TrapContext};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
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
        fdtable::FdTable,
        path::Path,
        stdio::{Stdin, Stdout},
    },
    include::{
        process::auxv::{AuxEntry, AT_EXECFN, AT_NULL, AT_RANDOM},
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
    task::{manager::add_new_process, taskid::tid_alloc},
};

/// shared between threads
type Shared<T> = Arc<SpinLock<T>>;

/// mutable resources mostly used in current thread
/// but, it could be accessed by other threads through process manager
/// so lock it with spinlock
type Mutable<T> = SpinLock<T>;

/// read-only resources, could be shared safely through threads
type Immutable<T> = T;

/// only used in current thread, mutable resources without lock
/// SAFETY: these resources won't be shared with other threads
type ThreadOnly<T> = SyncUnsafeCell<T>;

/// task control block inner
/// it is protected by a spinlock to assure its atomicity
/// so there's no need to use any lock in this struct
pub struct PCB {
    // paternity
    // assertion: only when the task is group leader, it can have children
    pub children: Vec<Arc<Task>>,        // children tasks
    pub zombie_children: Vec<Arc<Task>>, // zombie children
    pub parent: Option<Weak<Task>>,      // parent task, weak ptr

    // task status
    pub status: TaskStatus,  // task status
    pub exit_code: ExitCode, // exit code

    // signal structs
    pub pending_sigs: SigPending,        // pending signals
    pub sig_stack: Option<SigAltStack>,  // signal alternate stack
    pub ucontext_ptr: UserPtr<UContext>, // ucontext pointer
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
        }
    }
}

pub struct TCB {
    pub clear_child_tid: Option<usize>, // clear tid address
}

impl Default for TCB {
    fn default() -> Self {
        Self {
            clear_child_tid: None,
        }
    }
}

/// task control block for a coroutine,
/// a.k.a thread in current project structure
pub struct Task {
    // immutable
    tid: Immutable<TidTracer>, // task id, with lifetime holded
    tgid: Immutable<TGID>,     // task group id, aka pid
    pgid: Immutable<PGID>,     // process group id

    // mutable
    pcb: Mutable<PCB>, // task control block inner, protected by lock

    // thread only / once init
    waker: ThreadOnly<Option<Waker>>, // waker for the task
    trap_cx: ThreadOnly<TrapContext>, // trap context
    tcb: ThreadOnly<TCB>,             // thread control block

    // shared
    fd_table: Shared<FdTable>,         // file descriptor table
    cwd: Shared<Path>,                 // current work directory
    sa_list: Shared<SigActionList>,    // signal action list, saves signal handler
    memory_set: Shared<MemorySet>,     // memory set for the task
    thread_group: Shared<ThreadGroup>, // thread group

    // others
    pub sched_entity: SchedEntity, // sched entity for schedule
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
    pub fn pgid(&self) -> PGID {
        self.pgid
    }
    pub fn get_tg_leader(&self) -> Weak<Task> {
        self.thread_group
            .lock()
            .0
            .get(&self.tgid)
            .unwrap()
            .to_owned()
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
    pub fn thread_group_map<T>(&self, f: impl FnOnce(&mut ThreadGroup) -> T) -> T {
        f(&mut self.thread_group.lock())
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
        unsafe { &(*self.trap_cx.get()) }
    }
    #[inline(always)]
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        unsafe { &mut (*self.trap_cx.get()) }
    }

    /// signal info: sigaction list
    pub fn sa_list(&self) -> SpinLockGuard<SigActionList> {
        self.sa_list.lock()
    }

    /// get waker
    pub fn waker(&self) -> &Option<Waker> {
        unsafe { &(*self.waker.get()) }
    }
    /// set waker
    pub fn set_waker(&self, waker: Waker) {
        unsafe { (*self.waker.get()) = Some(waker) };
    }
    /// wake self up
    pub fn wake_unchecked(&self) {
        self.waker().as_ref().unwrap().wake_by_ref();
    }

    /// tcb
    pub fn tcb(&self) -> &TCB {
        self.tcb.as_ref()
    }
    pub fn tcb_mut(&self) -> &mut TCB {
        self.tcb.as_ref_mut()
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
}

// process implementation
impl Task {
    /// create new process from elf
    pub fn new_process(elf: ElfMemoryInfo) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
            auxs: _,
        } = elf;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid = tid_alloc();
        let tgid = tid.0;
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            pgid: 0,
            pcb: SpinLock::new(PCB::default()),
            thread_group: Arc::new(SpinLock::new(ThreadGroup::new())),
            memory_set: Arc::new(SpinLock::new(memory_set)),
            trap_cx: SyncUnsafeCell::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            sched_entity: SchedEntity::new_bare(INIT_PROCESS_ID),
            fd_table: Arc::new(SpinLock::new(FdTable::new())),
            cwd: Arc::new(SpinLock::new(Path::from(String::from("/")))),
            sa_list: Arc::new(SpinLock::new(SigActionList::new())),
            waker: SyncUnsafeCell::new(None),
            tcb: ThreadOnly::new(TCB {
                ..Default::default()
            }),
        });
        task.trap_context_mut()[TrapArgs::SP] -= 16;
        add_new_process(&task);
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// init user stack
    ///
    /// stack construction
    /// +---------------------------+
    /// | argc                      | <-- sp (lower address)
    /// | *argv                     |
    /// | *envp                     |
    /// | Padding (16-byte align)   |
    /// +---------------------------+
    /// | argv[0]                   |
    /// | argv[1]                   |
    /// | ...                       |
    /// | NULL (argv terminator)    |
    /// +---------------------------+
    /// | envp[0]                   |
    /// | envp[1]                   |
    /// | ...                       |
    /// | NULL (envp terminator)    |
    /// +---------------------------+
    /// | auxv[0].key, auxv[0].val  |
    /// | auxv[1].key, auxv[1].val  |
    /// | ...                       |
    /// | NULL (auxv terminator)    | <-- stack base (higher address)
    /// +---------------------------+
    pub fn init_user_stack(
        &self,
        user_sp: usize,
        args: Vec<String>,        // argv & argc
        envs: Vec<String>,        // env vec
        auxs: &mut Vec<AuxEntry>, // aux vec
    ) -> (usize, usize, usize, usize) {
        trace!("[init_user_stack] start");

        // user stack pointer
        let mut user_sp = user_sp;
        trace!("user_sp: {:#x}", user_sp);
        // argument vector
        let mut argv = vec![0; args.len() + 1];
        // environment pointer, end with NULL
        let mut envp = vec![0; envs.len() + 1];

        // === push args ===
        trace!("[init_user_stack] push args: {:?}", args);
        for (i, s) in args.iter().enumerate() {
            let len = s.len();
            user_sp -= len + 1;
            let p = user_sp as *mut u8;
            argv[i] = user_sp;
            unsafe {
                p.copy_from(s.as_ptr(), len);
                *((p as usize + len) as *mut u8) = 0;
            }
        }
        user_sp -= user_sp % core::mem::size_of::<usize>();

        // === push env ===
        trace!("[init_user_stack] push envs: {:?}", envs);
        for (i, s) in envs.iter().enumerate() {
            let len = s.len();
            user_sp -= len + 1;
            let p: *mut u8 = user_sp as *mut u8;
            envp[i] = user_sp;
            unsafe {
                p.copy_from(s.as_ptr(), len);
                *((p as usize + len) as *mut u8) = 0;
            }
        }
        // terminator: envp end with NULL
        envp[envs.len()] = 0;
        user_sp -= user_sp % core::mem::align_of::<usize>();

        // === push auxs ===
        trace!("[init_user_stack] push auxs");
        // random (16 bytes aligned, always 0 here)
        user_sp -= 16;
        auxs.push(AuxEntry(AT_RANDOM, user_sp as usize));
        user_sp -= user_sp % 16;
        // execfn, file name
        if !argv.is_empty() {
            auxs.push(AuxEntry(AT_EXECFN, argv[0] as usize)); // file name
        }
        // terminator: auxv end with AT_NULL
        auxs.push(AuxEntry(AT_NULL, 0 as usize)); // end
        trace!("[init_user_stack] auxs: {:?}", auxs);

        // construct auxv
        trace!("[init_user_stack] construct auxv");
        let auxs_len = auxs.len() * core::mem::size_of::<AuxEntry>();
        user_sp -= auxs_len;
        // let auxv_base = user_sp;
        for i in 0..auxs.len() {
            unsafe {
                *((user_sp + i * core::mem::size_of::<AuxEntry>()) as *mut usize) = auxs[i].0;
                *((user_sp + i * core::mem::size_of::<AuxEntry>() + core::mem::size_of::<usize>())
                    as *mut usize) = auxs[i].1;
            }
        }

        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp;

            trace!(
                "[init_user_stack] sp {:#x}, push_slice: {:#x?}",
                sp,
                unsafe { core::slice::from_raw_parts(sp as *const usize, slice.len()) }
            );
        }

        // construct envp
        trace!("[init_user_stack] construct envp, data: {:#x?}", envp);
        push_slice(&mut user_sp, envp.as_slice());
        let envp_base = user_sp;

        // push argv
        trace!("[init_user_stack] push argv, data: {:#x?}", argv);
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;

        // push argc, argv, envp
        let argc = args.len();
        push_slice(&mut user_sp, &[argc, argv_base, envp_base]);

        // return value: sp, argc, argv, envp
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            let (ms, _) = self.memory_set().lock().clone_cow();
            Arc::new(SpinLock::new(ms))
        };

        // TODO: CloneFlags::SIGHAND

        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            trace!("fd table info cloned");
            let tmp = Arc::new(SpinLock::new(self.fd_table.lock().clone()));
            let mut guard = tmp.lock();
            // todo: maybe needn't to realloc STD_IN
            guard.table[STD_IN] = Some(Arc::new(Stdin));
            guard.table[STD_OUT] = Some(Arc::new(Stdout::new()));
            guard.table[STD_ERR] = Some(Arc::new(Stdout::new()));
            drop(guard);
            tmp
        };

        let res = if flags.contains(CloneFlags::THREAD) {
            // fork as a new thread
            debug!("fork new thread");
            let new_tid = tid_alloc();
            let tid_val = new_tid.0;
            let new_thread = Arc::new(Self {
                tid: new_tid,
                tgid: self.tgid.clone(),
                pgid: self.pgid.clone(),
                thread_group: self.thread_group.clone(),
                pcb: SpinLock::new(PCB {
                    parent: self.pcb.lock().parent.clone(),
                    ..Default::default()
                }),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                cwd: self.cwd.clone(),
                sa_list: self.sa_list.clone(),
                waker: SyncUnsafeCell::new(None),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
            });
            self.thread_group.lock().insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            let new_tgid = new_tid.0;
            trace!("fork new process, tgid: {}", new_tgid);
            let new_process = Arc::new(Self {
                tid: new_tid,
                tgid: new_tgid,
                pgid: self.pgid.clone(),
                thread_group: Arc::new(SpinLock::new(ThreadGroup::new())),
                pcb: SpinLock::new(PCB {
                    parent: Some(self.get_tg_leader()),
                    ..Default::default()
                }),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                sched_entity: self.sched_entity.data_clone(new_tgid),
                fd_table,
                cwd: Arc::new(SpinLock::new(self.cwd().clone())),
                sa_list: Arc::new(SpinLock::new(SigActionList::new())),
                waker: SyncUnsafeCell::new(None),
                tcb: ThreadOnly::new(TCB {
                    ..Default::default()
                }),
            });
            add_new_process(&new_process);
            self.pcb().children.push(new_process.clone());
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
            elf_entry,
            user_sp,
            mut auxs,
        } = MemorySet::load_from_path(path).await;
        self.delete_children();
        memory_set.memory_activate();
        self.change_memory_set(memory_set);
        trace!("init usatck");
        let (user_sp, argc, argv_base, envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        self.trap_context_mut()
            .update_cx(elf_entry, user_sp, argc, argv_base, envp_base);
        // debug!("trap_context: {:#x?}", self.trap_context());
        // FIXME: close fd table, reset sigactions
        Ok(())
    }
}

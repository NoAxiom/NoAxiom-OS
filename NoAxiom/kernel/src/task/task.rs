//! # Task

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{
    sync::atomic::{AtomicI32, AtomicUsize, Ordering},
    task::Waker,
};

use arch::{Exception, TrapContext};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{LockGuard, SpinLock, SpinLockGuard},
};

use super::{
    manager::ThreadGroup,
    status::{AtomicSuspendReason, AtomicTaskStatus, SuspendReason, TaskStatus},
    taskid::{TidTracer, TGID, TID},
};
use crate::{
    config::{mm::USER_HEAP_SIZE, task::INIT_PROCESS_ID},
    constant::fs::{STD_ERR, STD_IN, STD_OUT},
    fs::{
        fdtable::FdTable,
        path::Path,
        stdio::{Stdin, Stdout},
    },
    include::{
        mm::{MmapFlags, MmapProts},
        process::auxv::{AuxEntry, AT_EXECFN, AT_NULL, AT_RANDOM},
        result::Errno,
        sched::CloneFlags,
        signal::sig_set::SigMask,
    },
    ipc::signal::{pending_sigs::PendingSigs, sa_list::SigActionList},
    mm::{
        address::VirtAddr,
        memory_set::{ElfMemoryInfo, MemorySet},
    },
    return_errno,
    sched::sched_entity::SchedEntity,
    syscall::{SysResult, SyscallResult},
    task::{manager::add_new_process, taskid::tid_alloc},
};

/// process resources info
pub struct ProcessInfo {
    /// children tasks, holds lifetime
    pub children: Vec<Arc<Task>>,

    /// parent task, weak ptr
    pub parent: Option<Weak<Task>>,

    /// current work directory
    pub cwd: Path,
}

pub struct SignalInfo {
    /// pending signals
    pending_sigs: Arc<SpinLock<PendingSigs>>,

    /// signal action list
    sa_list: Arc<SpinLock<SigActionList>>,

    /// signal mask
    sig_mask: SyncUnsafeCell<SigMask>,
    //
    // /// signal ucontext
    // sig_ucontext_cx: AtomicUsize,
    //
    // /// signal stack
    // pub sigstack: Option<SignalStack>,
}

impl SignalInfo {
    pub fn new(
        pending_sigs: Option<&Arc<SpinLock<PendingSigs>>>,
        sa_list: Option<&Arc<SpinLock<SigActionList>>>,
    ) -> Self {
        Self {
            pending_sigs: pending_sigs
                .map(|p| p.clone())
                .unwrap_or_else(|| Arc::new(SpinLock::new(PendingSigs::new()))),
            sa_list: sa_list
                .map(|p| p.clone())
                .unwrap_or_else(|| Arc::new(SpinLock::new(SigActionList::new()))),
            sig_mask: SyncUnsafeCell::new(SigMask::empty()),
            // sig_ucontext_cx: SyncUnsafeCell::new(SigContext::empty()),
        }
    }
}

/// task control block for a coroutine,
/// a.k.a thread in current project structure
pub struct Task {
    /// task id
    tid: TidTracer,

    /// task group id, aka pid
    tgid: Arc<AtomicUsize>,

    /// process group id
    pgid: Arc<AtomicUsize>,

    /// thread group tracer
    pub thread_group: Arc<SpinLock<ThreadGroup>>,

    /// process control block ptr,
    /// also belongs to other threads
    pcb: Arc<SpinLock<ProcessInfo>>,

    /// memory set for task
    /// explanation:
    /// SyncUnsafeCell: allow to change memory set in immutable context (bug?)
    /// todo: is SyncUnsafeCell correct??? might cause inter-core sync bugs
    /// Arc: allow to share memory set between tasks, also provides refcount
    /// SpinLock: allow to lock memory set in multi-core context
    // pub memory_set: SyncUnsafeCell<Arc<SpinLock<MemorySet>>>,
    pub memory_set: Arc<SpinLock<MemorySet>>,

    /// trap context,
    /// contains stack ptr, registers, etc.
    trap_cx: SyncUnsafeCell<TrapContext>,

    /// task status: ready / running / zombie
    status: AtomicTaskStatus,

    /// task suspend reason
    suspend_reason: AtomicSuspendReason,

    /// schedule entity for schedule
    pub sched_entity: SchedEntity,

    /// task exit code
    exit_code: AtomicI32,

    /// file descriptor table
    fd_table: Arc<SpinLock<FdTable>>,

    // signal info
    signal_info: SignalInfo,

    /// waker
    waker: SyncUnsafeCell<Option<Waker>>,
}

/// user tasks
/// - usage: wrap it in Arc<Task>
impl Task {
    /// tid
    #[inline(always)]
    pub fn tid(&self) -> TID {
        self.tid.0
    }
    pub fn tgid(&self) -> TGID {
        self.tgid.load(Ordering::SeqCst)
    }
    pub fn pgid(&self) -> usize {
        self.pgid.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn is_group_leader(&self) -> bool {
        self.tid() == self.tgid()
    }

    /// status
    #[inline(always)]
    pub fn status(&self) -> TaskStatus {
        self.status.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_status(&self, status: TaskStatus) {
        self.status.store(status, Ordering::SeqCst);
    }
    #[inline(always)]
    pub fn is_zombie(&self) -> bool {
        self.status() == TaskStatus::Zombie
    }
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.status() == TaskStatus::Running
    }
    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        self.status() == TaskStatus::Runnable
    }
    #[inline(always)]
    pub fn is_suspend(&self) -> bool {
        self.status() == TaskStatus::Suspend
    }

    // suspend reason
    #[inline(always)]
    pub fn suspend_reason(&self) -> SuspendReason {
        self.suspend_reason.load(Ordering::SeqCst)
    }
    pub fn set_suspend_reason(&self, reason: SuspendReason) {
        self.suspend_reason.store(reason, Ordering::SeqCst);
    }

    /// exit code
    #[inline(always)]
    pub fn exit_code(&self) -> i32 {
        self.exit_code.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn set_exit_code(&self, exit_code: i32) {
        self.exit_code.store(exit_code, Ordering::SeqCst);
    }

    /// memory set
    #[inline(always)]
    pub fn memory_set(&self) -> &Arc<SpinLock<MemorySet>> {
        // unsafe { &(*self.memory_set.get()) }
        &self.memory_set
    }
    #[inline(always)]
    pub unsafe fn memory_activate(&self) {
        unsafe { self.memory_set().lock().activate() };
    }
    /// get token from memory set
    #[inline(always)]
    pub fn token(&self) -> usize {
        self.memory_set().lock().token()
    }
    /// change current memory set
    pub fn change_memory_set(&self, memory_set: MemorySet) {
        // unsafe { (*self.memory_set.get()) = Arc::new(SpinLock::new(memory_set)) };
        let mut ms = self.memory_set.lock();
        *ms = memory_set;
    }

    pub async fn memory_validate(
        self: &Arc<Self>,
        addr: usize,
        exception: Option<Exception>,
    ) -> SysResult<()> {
        trace!("[memory_validate] check at addr: {:#x}", addr);
        let vpn = VirtAddr::from(addr).floor();
        let mut ms = self.memory_set().lock();
        let pte = ms.page_table().translate_vpn(vpn);
        ms.validate(vpn, exception, pte).await
    }

    /// get pcb
    #[inline(always)]
    pub fn pcb(&self) -> SpinLockGuard<ProcessInfo> {
        self.pcb.lock()
    }

    /// get fd_table
    #[inline(always)]
    pub fn fd_table(&self) -> SpinLockGuard<FdTable> {
        self.fd_table.lock()
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
        self.signal_info.sa_list.lock()
    }
    /// signal info: pending signals
    pub fn pending_sigs(&self) -> SpinLockGuard<PendingSigs> {
        self.signal_info.pending_sigs.lock()
    }
    /// signal info: signal mask
    pub fn sig_mask(&self) -> &SigMask {
        unsafe { &(*self.signal_info.sig_mask.get()) }
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
    pub fn wake_up(&self) {
        if let Some(waker) = self.waker().as_ref() {
            waker.wake_by_ref();
        }
    }

    /// create new process from elf
    pub async fn new_process(path: Path) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
            auxs,
        } = MemorySet::load_from_path(path.clone()).await;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid = tid_alloc();
        let tgid = Arc::new(AtomicUsize::new(tid.0));
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            pgid: Arc::new(AtomicUsize::new(0)),
            thread_group: Arc::new(SpinLock::new(ThreadGroup::new())),
            pcb: Arc::new(SpinLock::new(ProcessInfo {
                children: Vec::new(),
                parent: None,
                cwd: path,
            })),
            memory_set: Arc::new(SpinLock::new(memory_set)),
            trap_cx: SyncUnsafeCell::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            status: AtomicTaskStatus::new(TaskStatus::Runnable),
            suspend_reason: AtomicSuspendReason::new(SuspendReason::None),
            exit_code: AtomicI32::new(0),
            sched_entity: SchedEntity::new_bare(INIT_PROCESS_ID),
            fd_table: Arc::new(SpinLock::new(FdTable::new())),
            signal_info: SignalInfo::new(None, None),
            waker: SyncUnsafeCell::new(None),
        });
        add_new_process(&task);
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// init user stack
    ///
    /// stack construction
    /// +---------------------------+
    /// | Padding (16-byte align)   | <-- sp
    /// +---------------------------+
    /// | argc                      |
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
    /// | NULL (auxv terminator)    |
    /// +---------------------------+
    pub fn init_user_stack(
        &self,
        user_sp: usize,
        args: Vec<String>,        // argv & argc
        envs: Vec<String>,        // env vec
        auxs: &mut Vec<AuxEntry>, // aux vec
    ) -> (usize, usize, usize, usize) {
        trace!("[init_user_stack] start");

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

        // construct envp
        trace!("[init_user_stack] construct envp, data: {:#x?}", envp);
        push_slice(&mut user_sp, envp.as_slice());
        let envp_base = user_sp;

        // push argv
        trace!("[init_user_stack] push argv, data: {:#x?}", argv);
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;

        // push argc
        trace!("[init_user_stack] push argc");
        push_slice(&mut user_sp, &[args.len()]);

        // return value: sp, argc, argv, envp
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            Arc::new(SpinLock::new(self.memory_set().lock().clone_cow()))
        };

        // TODO: CloneFlags::SIGHAND

        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            debug!("fd table info cloned");
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
                pcb: self.pcb.clone(),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: AtomicTaskStatus::new(TaskStatus::Runnable),
                suspend_reason: AtomicSuspendReason::new(SuspendReason::None),
                exit_code: AtomicI32::new(0),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                signal_info: SignalInfo::new(
                    Some(&self.signal_info.pending_sigs),
                    Some(&self.signal_info.sa_list),
                ),
                waker: SyncUnsafeCell::new(None),
            });
            self.thread_group.lock().insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            let tid_val = new_tid.0;
            trace!("fork new process, tgid: {}", tid_val);
            let mut parent_pcb = self.pcb();
            let new_process = Arc::new(Self {
                tid: new_tid,
                tgid: Arc::new(AtomicUsize::new(tid_val)),
                pgid: self.pgid.clone(),
                thread_group: self.thread_group.clone(),
                pcb: Arc::new(SpinLock::new(ProcessInfo {
                    children: Vec::new(),
                    parent: Some(Arc::downgrade(self)),
                    cwd: parent_pcb.cwd.clone(),
                })),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: AtomicTaskStatus::new(TaskStatus::Runnable),
                suspend_reason: AtomicSuspendReason::new(SuspendReason::None),
                exit_code: AtomicI32::new(0),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                signal_info: SignalInfo::new(None, None),
                waker: SyncUnsafeCell::new(None),
            });
            add_new_process(&new_process);
            parent_pcb.children.push(new_process.clone());
            new_process
        };
        res
    }

    /// execute
    pub async fn exec(
        self: &Arc<Self>,
        path: Path,
        mut args: Vec<String>,
        mut envs: Vec<String>,
    ) -> SysResult<()> {
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
            mut auxs,
        } = MemorySet::load_from_path(path).await;
        self.delete_children();
        unsafe { memory_set.activate() };
        self.change_memory_set(memory_set);
        trace!("init usatck");
        let (user_sp, argc, argv_base, envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        self.trap_context_mut()
            .update_cx(elf_entry, user_sp, argc, argv_base, envp_base);
        // debug!("trap_context: {:#x?}", self.trap_context());
        trace!(
            "trap_context: tid: {}, A0: {:#x}, A1: {:#x}, A2: {:#x}",
            self.tid(),
            self.trap_context().user_reg[10],
            self.trap_context().user_reg[11],
            self.trap_context().user_reg[12],
        );
        // TODO: close fd table, reset sigactions
        Ok(())
    }

    /// exit current task
    pub fn exit(&self, exit_code: i32) {
        if self.is_group_leader() {
            self.set_exit_code(exit_code);
        }
        self.set_status(TaskStatus::Zombie);
    }

    pub fn grow_brk(self: &Arc<Self>, new_brk: usize) -> SyscallResult {
        let mut memory_set: LockGuard<'_, MemorySet> = self.memory_set.lock();
        let grow_size = new_brk - memory_set.user_brk;
        trace!(
            "[grow_brk] start: {:#x}, old_brk: {:#x}, new_brk: {:#x}",
            memory_set.user_brk_start,
            memory_set.user_brk,
            new_brk
        );
        if grow_size > 0 {
            trace!("[grow_brk] expanded");
            let growed_addr: usize = memory_set.user_brk + grow_size as usize;
            let limit = memory_set.user_brk_start + USER_HEAP_SIZE;
            if growed_addr > limit {
                return_errno!(Errno::ENOMEM);
            }
            memory_set.user_brk = growed_addr;
        } else {
            trace!("[grow_brk] shrinked");
            if new_brk < memory_set.user_brk_start {
                return_errno!(Errno::EINVAL);
            }
            memory_set.user_brk = new_brk;
        }
        memory_set.brk_grow(VirtAddr(new_brk).ceil());
        Ok(memory_set.user_brk as isize)
    }

    pub fn mmap(
        &self,
        addr: usize,
        length: usize,
        prot: MmapProts,
        flags: MmapFlags,
        fd: isize,
        offset: usize,
    ) -> SysResult<usize> {
        // check file validity, and fetch file from fd_table
        let fd_table = self.fd_table();
        if !flags.contains(MmapFlags::MAP_ANONYMOUS)
            && (fd as usize >= fd_table.table.len() || fd_table.table[fd as usize].is_none())
        {
            return Err(Errno::EBADF);
        }
        let fd_table = fd_table.table.clone();

        // get start_va
        let mut memory_set = self.memory_set().lock();
        let mut start_va = VirtAddr::from(addr);
        if addr == 0 {
            start_va = memory_set.mmap_manager.mmap_top;
        }

        // if contains fix flag, should remove the existing mapping
        if flags.contains(MmapFlags::MAP_FIXED) {
            start_va = VirtAddr::from(addr);
            memory_set.mmap_manager.remove(start_va, length);
        }

        // get target file
        let file = if flags.contains(MmapFlags::MAP_ANONYMOUS) {
            None
        } else {
            fd_table[fd as usize].clone()
        };

        // push mmap range (without immediate mapping)
        memory_set
            .mmap_manager
            .insert(start_va, length, prot, flags, offset, file);
        Ok(start_va.0)
    }
}

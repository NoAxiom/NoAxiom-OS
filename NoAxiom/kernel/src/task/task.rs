//! # Task

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{
    sync::atomic::{AtomicI32, AtomicUsize, Ordering},
    task::Waker,
};

use arch::{Arch, ArchMemory, ArchTrapContext, TrapContext, TrapType};
use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
};

use super::{
    manager::ThreadGroup,
    process_info::ProcessInfo,
    status::TaskStatus,
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
    },
    mm::{
        address::VirtAddr,
        memory_set::{ElfMemoryInfo, MemorySet, MemorySpace},
        page_table::PageTable,
        validate::validate,
    },
    return_errno,
    sched::sched_entity::SchedEntity,
    signal::{
        sig_action::SigActionList, sig_control_block::SignalControlBlock, sig_pending::SigPending,
        sig_set::SigMask,
    },
    syscall::{SysResult, SyscallResult},
    task::{manager::add_new_process, taskid::tid_alloc},
};

/// task control block for a coroutine,
/// a.k.a thread in current project structure
pub struct Task {
    /// [th] task id
    tid: TidTracer,

    /// [pr] task group id, aka pid
    tgid: Arc<AtomicUsize>,

    /// [pr] process group id
    pgid: Arc<AtomicUsize>,

    /// [pr] thread group tracer
    pub thread_group: Arc<SpinLock<ThreadGroup>>,

    /// [pr] process control block ptr,
    pcb: Arc<SpinLock<ProcessInfo>>,

    /// [pr] memory set for task
    memory_space: MemorySpace,

    /// [th] trap context,
    /// contains stack ptr, registers, etc.
    trap_cx: SyncUnsafeCell<TrapContext>,

    /// [th] task status: ready / running / zombie
    pub status: Arc<SpinLock<TaskStatus>>,

    /// [th] schedule entity for schedule
    pub sched_entity: SchedEntity,

    /// [th?] task exit code
    exit_code: AtomicI32,

    /// [pr] file descriptor table
    fd_table: Arc<SpinLock<FdTable>>,

    /// [pr] current work directory
    cwd: Arc<SpinLock<Path>>,

    /// [pr/th] signal info
    signal: SignalControlBlock,

    /// [th] waker
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
    pub fn status(&self) -> SpinLockGuard<TaskStatus> {
        self.status.lock()
    }
    #[inline(always)]
    pub fn get_status(&self) -> TaskStatus {
        *self.status.lock()
    }
    #[allow(unused)]
    #[inline(always)]
    pub fn set_status(&self, status: TaskStatus) {
        *self.status.lock() = status;
    }
    #[allow(unused)]
    pub fn set_suspend(&self) {
        self.set_status(TaskStatus::Suspend);
    }
    #[allow(unused)]
    pub fn set_runnable(&self) {
        self.set_status(TaskStatus::Runnable);
    }
    #[allow(unused)]
    pub fn is_suspend(&self) -> bool {
        self.get_status() == TaskStatus::Suspend
    }
    #[allow(unused)]
    pub fn is_runnable(&self) -> bool {
        self.get_status() == TaskStatus::Runnable
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
        &self.memory_space.memory_set
    }
    #[inline(always)]
    pub fn memory_activate(&self) {
        self.memory_space.memory_activate();
    }
    /// change current memory set
    pub fn change_memory_set(&self, memory_set: MemorySet) {
        // unsafe { (*self.memory_set.get()) = Arc::new(SpinLock::new(memory_set)) };
        let mut ms = self.memory_set().lock();
        self.memory_space.change_root_ppn(memory_set.root_ppn().0);
        *ms = memory_set;
    }

    pub async fn memory_validate(
        self: &Arc<Self>,
        addr: usize,
        trap_type: Option<TrapType>,
    ) -> SysResult<()> {
        trace!("[memory_validate] check at addr: {:#x}", addr);
        let vpn = VirtAddr::from(addr).floor();
        let pt = PageTable::from_ppn(Arch::current_root_ppn());
        validate(self.memory_set(), vpn, trap_type, pt.translate_vpn(vpn)).await
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

    pub fn signal(&self) -> &SignalControlBlock {
        &self.signal
    }
    /// signal info: sigaction list
    pub fn sa_list(&self) -> SpinLockGuard<SigActionList> {
        self.signal.sa_list.lock()
    }
    /// signal info: pending signals
    pub fn pending_sigs(&self) -> SpinLockGuard<SigPending> {
        self.signal.pending_sigs.lock()
    }
    /// signal info: signal mask
    pub fn sig_mask(&self) -> &SigMask {
        unsafe { &(*self.signal.sig_mask.get()) }
    }
    pub fn sig_mask_mut(&self) -> &mut SigMask {
        unsafe { &mut (*self.signal.sig_mask.get()) }
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
    pub fn wake(&self) {
        self.waker().as_ref().unwrap().wake_by_ref();
    }

    /// create new process from elf
    pub async fn new_process(path: Path) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
            auxs: _,
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
                zombie_children: Vec::new(),
                parent: None,
                wait_req: false,
            })),
            memory_space: MemorySpace {
                root_ppn: SyncUnsafeCell::new(memory_set.root_ppn().0),
                memory_set: Arc::new(SpinLock::new(memory_set)),
            },
            trap_cx: SyncUnsafeCell::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            status: Arc::new(SpinLock::new(TaskStatus::Runnable)),
            exit_code: AtomicI32::new(0),
            sched_entity: SchedEntity::new_bare(INIT_PROCESS_ID),
            fd_table: Arc::new(SpinLock::new(FdTable::new())),
            cwd: Arc::new(SpinLock::new(path)),
            signal: SignalControlBlock::new(None),
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
        let (memory_set, root_ppn) = if flags.contains(CloneFlags::VM) {
            (
                self.memory_set().clone(),
                SyncUnsafeCell::new(self.memory_space.root_ppn()),
            )
        } else {
            let (ms, root_ppn) = self.memory_set().lock().clone_cow();
            (Arc::new(SpinLock::new(ms)), SyncUnsafeCell::new(root_ppn))
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
                pcb: self.pcb.clone(),
                memory_space: MemorySpace {
                    root_ppn,
                    memory_set,
                },
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: Arc::new(SpinLock::new(TaskStatus::Runnable)),
                exit_code: AtomicI32::new(0),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                cwd: self.cwd.clone(),
                signal: SignalControlBlock::new(Some(&self.signal.sa_list)),
                waker: SyncUnsafeCell::new(None),
            });
            self.thread_group.lock().insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            let tid_val = new_tid.0;
            trace!("fork new process, tgid: {}", tid_val);
            let new_process = Arc::new(Self {
                tid: new_tid,
                tgid: Arc::new(AtomicUsize::new(tid_val)),
                pgid: self.pgid.clone(),
                thread_group: Arc::new(SpinLock::new(ThreadGroup::new())),
                pcb: Arc::new(SpinLock::new(ProcessInfo {
                    children: Vec::new(),
                    zombie_children: Vec::new(),
                    parent: Some(Arc::downgrade(self)),
                    wait_req: false,
                })),
                memory_space: MemorySpace {
                    memory_set,
                    root_ppn,
                },
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: Arc::new(SpinLock::new(TaskStatus::Runnable)),
                exit_code: AtomicI32::new(0),
                sched_entity: self.sched_entity.data_clone(tid_val),
                fd_table,
                cwd: Arc::new(SpinLock::new(self.cwd().clone())),
                signal: SignalControlBlock::new(None),
                waker: SyncUnsafeCell::new(None),
            });
            add_new_process(&new_process);
            self.pcb().children.push(new_process.clone());
            new_process
        };
        res
    }

    /// execute
    pub async fn exec(
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
        // TODO: close fd table, reset sigactions
        Ok(())
    }

    /// exit current task
    pub fn terminate(&self, exit_code: i32) {
        if self.is_group_leader() {
            self.set_exit_code(exit_code);
        }
        self.set_status(TaskStatus::Terminated);
    }

    pub fn grow_brk(self: &Arc<Self>, new_brk: usize) -> SyscallResult {
        let mut memory_set = self.memory_set().lock();
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

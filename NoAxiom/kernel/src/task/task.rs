//! # Task

use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use ksync::{
    cell::SyncUnsafeCell,
    mutex::{SpinLock, SpinLockGuard},
};
use riscv::register::scause::Exception;

use super::taskid::TidTracer;
use crate::{
    fs::{fdtable::FdTable, path::Path},
    mm::memory_set::{ElfMemoryInfo, MemorySet},
    nix::{
        auxv::{AuxEntry, AT_EXECFN, AT_NULL, AT_RANDOM},
        clone_flags::CloneFlags,
    },
    sched::sched_entity::SchedEntity,
    syscall::SyscallResult,
    task::taskid::tid_alloc,
    trap::TrapContext,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

/// process resources info
pub struct ProcessInfo {
    /// children tasks, holds lifetime
    children: Vec<Arc<Task>>,

    /// parent task, weak ptr
    parent: Option<Weak<Task>>,

    /// current work directory
    pub cwd: Path,
}

/// task control block for a coroutine,
/// a.k.a thread in current project structure
pub struct Task {
    /// task group id, aka process_id
    tgid: Arc<AtomicUsize>,

    /// task id, aka thread_id
    tid: TidTracer,

    /// process control block ptr,
    /// also belongs to other threads
    pcb: Arc<SpinLock<ProcessInfo>>,

    /// memory set for task
    /// it's a process resource as well
    ///
    /// explanation:
    /// SyncUnsafeCell: allow to change memory set in immutable context
    /// Arc: allow to share memory set between tasks, also provides refcount
    /// SpinLock: allow to lock memory set in multi-core context
    pub memory_set: SyncUnsafeCell<Arc<SpinLock<MemorySet>>>,

    /// trap context,
    /// contains stack ptr, registers, etc.
    trap_cx: SyncUnsafeCell<TrapContext>,

    /// task status: ready / running / zombie
    status: SyncUnsafeCell<TaskStatus>,

    /// schedule entity for schedule
    pub sched_entity: SchedEntity,

    /// task exit code
    exit_code: AtomicIsize,

    /// file descriptor table
    fd_table: Arc<SpinLock<FdTable>>,
}

/// user tasks
/// - usage: wrap it in Arc<Task>
#[allow(unused)]
impl Task {
    /// tid
    #[inline(always)]
    pub fn tid(&self) -> usize {
        self.tid.0
    }
    pub fn tgid(&self) -> usize {
        self.tgid.load(Ordering::SeqCst)
    }
    #[inline(always)]
    pub fn is_leader(&self) -> bool {
        self.tid() == self.tgid()
    }

    /// status
    #[inline(always)]
    pub fn status(&self) -> &TaskStatus {
        unsafe { &(*self.status.get()) }
    }
    #[inline(always)]
    pub fn status_mut(&self) -> &mut TaskStatus {
        unsafe { &mut (*self.status.get()) }
    }
    #[inline(always)]
    pub fn set_status(&self, status: TaskStatus) {
        *self.status_mut() = status;
    }
    #[inline(always)]
    pub fn is_zombie(&self) -> bool {
        *self.status() == TaskStatus::Zombie
    }
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        *self.status() == TaskStatus::Running
    }
    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        *self.status() == TaskStatus::Ready
    }

    /// exit code
    #[inline(always)]
    pub fn exit_code(&self) -> isize {
        self.exit_code.load(Ordering::Relaxed)
    }
    #[inline(always)]
    pub fn set_exit_code(&self, exit_code: isize) {
        self.exit_code.store(exit_code, Ordering::Relaxed);
    }

    /// memory set
    #[inline(always)]
    pub fn memory_set(&self) -> &Arc<SpinLock<MemorySet>> {
        unsafe { &(*self.memory_set.get()) }
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
        unsafe {
            (*self.memory_set.get()) = Arc::new(SpinLock::new(memory_set));
        }
    }

    pub fn memory_validate(
        self: &Arc<Self>,
        addr: usize,
        exception: Option<Exception>,
    ) -> SyscallResult {
        trace!("[memory_validate] check at addr: {:#x}", addr);
        self.memory_set().lock().validate(addr, exception)
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
            pcb: Arc::new(SpinLock::new(ProcessInfo {
                children: Vec::new(),
                parent: None,
                cwd: path,
            })),
            memory_set: SyncUnsafeCell::new(Arc::new(SpinLock::new(memory_set))),
            trap_cx: SyncUnsafeCell::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            status: SyncUnsafeCell::new(TaskStatus::Ready),
            exit_code: AtomicIsize::new(0),
            sched_entity: SchedEntity::new_bare(),
            fd_table: Arc::new(SpinLock::new(FdTable::new())),
        });
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
        debug!("[init_user_stack] start");

        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp;

            debug!(
                "[init_user_stack] sp {:#x}, push_slice: {:#x?}",
                sp,
                unsafe { core::slice::from_raw_parts(sp as *const usize, slice.len()) }
            );
        }

        // user stack pointer
        let mut user_sp = user_sp;
        info!("user_sp: {:#x}", user_sp);
        // argument vector
        let mut argv = vec![0; args.len() + 1];
        // environment pointer, end with NULL
        let mut envp = vec![0; envs.len() + 1];

        // === push args ===
        debug!("[init_user_stack] push args: {:?}", args);
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
        debug!("[init_user_stack] push envs: {:?}", envs);
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
        debug!("[init_user_stack] push auxs");
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
        debug!("[init_user_stack] auxs: {:?}", auxs);

        // construct auxv
        debug!("[init_user_stack] construct auxv");
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
        debug!("[init_user_stack] construct envp, data: {:#x?}", envp);
        push_slice(&mut user_sp, envp.as_slice());
        let envp_base = user_sp;

        // push argv
        debug!("[init_user_stack] push argv, data: {:#x?}", argv);
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;

        // push argc
        debug!("[init_user_stack] push argc");
        push_slice(&mut user_sp, &[args.len()]);

        // return value: sp, argc, argv, envp
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        // memory set clone
        let memory_set = SyncUnsafeCell::new(if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            Arc::new(SpinLock::new(self.memory_set().lock().clone_cow()))
        });

        // TODO: CloneFlags::SIGHAND
        // TODO: fd table (CloneFlags::FILES)
        // let fd = if flags.contains(CloneFlags::FILES) {
        // self.fd_table.clone()
        // } else {
        // self.fd_table.clone_cow()
        // };

        // TODO: push task into process/thread manager
        if flags.contains(CloneFlags::THREAD) {
            // fork as a new thread
            let task = Arc::new(Self {
                tgid: self.tgid.clone(),
                tid: tid_alloc(),
                pcb: self.pcb.clone(),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: SyncUnsafeCell::new(TaskStatus::Ready),
                exit_code: AtomicIsize::new(0),
                sched_entity: self.sched_entity.data_clone(),
                fd_table: self.fd_table.clone(),
            });
            task
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            trace!("fork new process, tid: {}", new_tid.0);
            let task = Arc::new(Self {
                tgid: Arc::new(AtomicUsize::new(new_tid.0)),
                tid: new_tid,
                pcb: Arc::new(SpinLock::new(ProcessInfo {
                    children: Vec::new(),
                    parent: Some(Arc::downgrade(self)),
                    cwd: self.pcb().cwd.clone(),
                })),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: SyncUnsafeCell::new(TaskStatus::Ready),
                exit_code: AtomicIsize::new(0),
                sched_entity: self.sched_entity.data_clone(),
                fd_table: self.fd_table.clone(),
            });
            task
        }
    }

    /// execute
    pub async fn exec(self: &Arc<Self>, path: Path, mut args: Vec<String>, mut envs: Vec<String>) {
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
            mut auxs,
        } = MemorySet::load_from_path(path).await;
        // TODO: delete child
        unsafe { memory_set.activate() };
        self.change_memory_set(memory_set);
        trace!("init usatck");
        args.push("hello".to_string());
        args.push("world".to_string());
        args.push("qwq".to_string());
        envs.push("PATH=/usr/bin".to_string());
        envs.push("test_for_execve".to_string());
        let (user_sp, argc, argv_base, envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        self.trap_context_mut()
            .update_cx(elf_entry, user_sp, argc, argv_base, envp_base);
        // debug!("trap_context: {:#x?}", self.trap_context());
        debug!(
            "trap_context: tid: {}, A0: {:#x}, A1: {:#x}, A2: {:#x}",
            self.tid(),
            self.trap_context().user_reg[10],
            self.trap_context().user_reg[11],
            self.trap_context().user_reg[12],
        );
        // TODO: close fd table, reset sigactions
    }

    /// exit current task
    pub fn exit(&self) {
        self.set_status(TaskStatus::Zombie);
    }
}

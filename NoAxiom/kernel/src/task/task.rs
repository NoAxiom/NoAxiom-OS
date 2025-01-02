//! # Task

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use riscv::asm::sfence_vma_all;

use super::taskid::TidTracer;
use crate::{
    fs::path::Path,
    mm::{
        address::VirtAddr,
        memory_set::{ElfMemoryInfo, MemorySet},
    },
    nix::{
        auxv::{AuxEntry, AT_EXECFN, AT_NULL, AT_RANDOM},
        clone_flags::CloneFlags,
    },
    sched::sched_entity::SchedEntity,
    sync::{cell::SyncUnsafeCell, mutex::SpinLock},
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
    pub memory_set: Arc<SpinLock<MemorySet>>,

    /// trap context,
    /// contains stack ptr, registers, etc.
    trap_cx: SyncUnsafeCell<TrapContext>,

    /// task status: ready / running / zombie
    status: SyncUnsafeCell<TaskStatus>,

    /// schedule entity for schedule
    pub sched_entity: SchedEntity,

    /// task exit code
    exit_code: AtomicIsize,
    // /// file descriptor
    // fd: Arc<SpinLock<FdTable>>,
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
    pub unsafe fn memory_activate(&self) {
        unsafe { self.memory_set.lock().activate() };
    }
    /// get token from memory set
    #[inline(always)]
    pub fn token(&self) -> usize {
        self.memory_set.lock().token()
    }
    /// change current memory set
    pub fn change_memory_set(&self, memory_set: MemorySet) {
        *self.memory_set.lock() = memory_set;
    }

    // TODO: add mmap check
    /// # memory validate
    /// Check if is the copy-on-write/lazy-alloc pages triggered the page fault.
    ///
    /// As for cow, clone pages for the writer(aka current task),
    /// but should keep original page as cow since it might still be shared.
    /// Note that if the reference count is one, there's no need to clone pages.
    ///
    /// As for lazy alloc, realloc pages for the task.
    /// Associated pages: stack, heap, mmap
    ///
    /// Return value: true if successfully handled lazy alloc or copy-on-write;
    ///               false if the page fault is not in any alloc area.
    ///
    /// usages: when any kernel allocation in user_space happens, call this fn;
    /// when user pagefault happens, call this func to check allocation.
    pub fn memory_validate(self: &Arc<Self>, addr: usize) -> bool {
        warn!("[memory_validate] check at addr: {:#x}", addr);
        let mut memory_set = self.memory_set.lock();
        let vpn = VirtAddr::from(addr).floor();
        if let Some(pte) = memory_set.page_table().translate_vpn(vpn) {
            let flags = pte.flags();
            if flags.is_cow() {
                memory_set.realloc_cow(vpn, pte);
                return true;
            } else if flags.is_valid() {
                error!("[check_lazy] pte is V but not COW, flags: {:?}", flags);
                return false;
            }
        } else if memory_set.user_stack_area.vpn_range.is_in_range(vpn) {
            trace!("page fault at lazy-alloc stack, realloc stack");
            memory_set.realloc_stack(vpn);
            trace!("stack reallocated");
            return true;
        } else if memory_set.user_heap_area.vpn_range.is_in_range(vpn) {
            memory_set.realloc_heap(vpn);
            return true;
        }
        error!("page fault at addr: {:#x}, but not in any alloc area", addr);
        false
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
    #[inline(always)]
    pub fn set_trap_context(&self, trap_context: TrapContext) {
        *self.trap_context_mut() = trap_context;
    }

    /// create new process from elf
    pub async fn new_process(path: Path) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
        } = MemorySet::load_from_path(path).await;
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
            })),
            memory_set: Arc::new(SpinLock::new(memory_set)),
            trap_cx: SyncUnsafeCell::new(TrapContext::app_init_cx(elf_entry, user_sp)),
            status: SyncUnsafeCell::new(TaskStatus::Ready),
            exit_code: AtomicIsize::new(0),
            sched_entity: SchedEntity::new_bare(),
        });
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    // TODO: WIP
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
        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp
        }

        // user stack pointer
        let mut user_sp = user_sp;
        // argument vector
        let mut argv = vec![0; args.len()];
        // environment pointer, end with NULL
        let mut envp = vec![0; envs.len() + 1];

        // === push args ===
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
        user_sp = user_sp % core::mem::align_of::<usize>();

        // === push auxs ===
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

        // construct auxv
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
        let len = envs.len() * core::mem::size_of::<usize>();
        user_sp -= len;
        let envp_base = user_sp;
        for i in 0..envs.len() {
            unsafe {
                *((envp_base + i * core::mem::size_of::<usize>()) as *mut usize) = envp[i];
            }
        }
        unsafe {
            *((envp_base + envs.len() * core::mem::size_of::<usize>()) as *mut usize) = 0;
        }

        // push argv
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;

        // push argc
        push_slice(&mut user_sp, &[args.len()]);
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        // memory set clone
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set.clone()
        } else {
            let res = Arc::new(SpinLock::new(self.memory_set.lock().clone_cow()));
            unsafe { sfence_vma_all() };
            res
        };

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
                })),
                memory_set,
                trap_cx: SyncUnsafeCell::new(self.trap_context().clone()),
                status: SyncUnsafeCell::new(TaskStatus::Ready),
                exit_code: AtomicIsize::new(0),
                sched_entity: self.sched_entity.data_clone(),
            });
            task
        }
    }

    /// execute
    pub async fn exec(self: &Arc<Self>, path: Path, args: Vec<String>, envs: Vec<String>) {
        let ElfMemoryInfo {
            memory_set,
            elf_entry,
            user_sp,
        } = MemorySet::load_from_path(path).await;
        // TODO: delete child
        unsafe { memory_set.activate() };
        self.change_memory_set(memory_set);
        // TODO: init ustack
        self.set_trap_context(TrapContext::app_init_cx(elf_entry, user_sp));
    }

    /// exit current task
    pub fn exit(&self) {
        self.set_status(TaskStatus::Zombie);
    }
}

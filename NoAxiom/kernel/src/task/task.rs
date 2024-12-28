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
    nix::clone_flags::CloneFlags,
    sched::sched_entity::SchedEntity,
    sync::{cell::SyncUnsafeCell, mutex::SpinMutex},
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

/// thread resources info
pub struct ThreadInfo {
    /// trap context,
    /// contains stack ptr, registers, etc.
    pub trap_context: TrapContext,
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
    pcb: Arc<SpinMutex<ProcessInfo>>,

    /// memory set for task
    /// it's a process resource as well
    pub memory_set: Arc<SpinMutex<MemorySet>>,

    /// thread control block ptr
    thread: SyncUnsafeCell<ThreadInfo>,

    /// task status: ready / running / zombie
    status: SyncUnsafeCell<TaskStatus>,

    /// schedule entity for schedule
    pub sched_entity: SchedEntity,

    /// task exit code
    exit_code: AtomicIsize,
    // /// file descriptor
    // fd: Arc<SpinMutex<FdTable>>,
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

    /// thread info
    #[inline(always)]
    pub fn thread(&self) -> &ThreadInfo {
        unsafe { &(*self.thread.get()) }
    }
    #[inline(always)]
    pub fn thread_mut(&self) -> &mut ThreadInfo {
        unsafe { &mut (*self.thread.get()) }
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

    /// Check if is the copy-on-write pages triggered the page fault.
    /// If it's true, clone pages for the writer(aka current task),
    /// but should keep original page as cow since it might still be shared.
    /// Note that if the reference count is one, there's no need to clone pages.
    /// return value: true if detected lazy alloc orcopy-on-write
    /// and cloned successfully
    pub fn handle_pagefault(self: &Arc<Self>, addr: usize) -> bool {
        let mut memory_set = self.memory_set.lock();
        let vpn = VirtAddr::from(addr).floor();
        if let Some(pte) = memory_set.page_table().translate_vpn(vpn) {
            let flags = pte.flags();
            if flags.is_cow() {
                memory_set.realloc_cow(vpn, pte);
                return true;
            } else if flags.is_valid() {
                warn!("[check_lazy] pte is V but not COW, flags: {:?}", flags);
                return false;
            }
        } else if memory_set.user_stack_area.vpn_range.is_in_range(vpn) {
            self.memory_set.lock().realloc_stack(vpn);
            return true;
        } else if memory_set.user_heap_area.vpn_range.is_in_range(vpn) {
            self.memory_set.lock().realloc_heap(vpn);
            return true;
        }
        false
    }

    /// trap context
    #[inline(always)]
    pub fn trap_context(&self) -> &TrapContext {
        &self.thread().trap_context
    }
    #[inline(always)]
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        &mut self.thread_mut().trap_context
    }
    #[inline(always)]
    pub fn set_trap_context(&self, trap_context: TrapContext) {
        self.thread_mut().trap_context = trap_context;
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
            pcb: Arc::new(SpinMutex::new(ProcessInfo {
                children: Vec::new(),
                parent: None,
            })),
            memory_set: Arc::new(SpinMutex::new(memory_set)),
            thread: SyncUnsafeCell::new(ThreadInfo {
                trap_context: TrapContext::app_init_cx(elf_entry, user_sp),
            }),
            status: SyncUnsafeCell::new(TaskStatus::Ready),
            exit_code: AtomicIsize::new(0),
            sched_entity: SchedEntity::new_bare(),
        });
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// fork
    pub fn fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        // memory set clone
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set.clone()
        } else {
            let res = Arc::new(SpinMutex::new(self.memory_set.lock().clone_cow()));
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
                thread: SyncUnsafeCell::new(ThreadInfo {
                    trap_context: self.trap_context().clone(),
                }),
                status: SyncUnsafeCell::new(TaskStatus::Ready),
                exit_code: AtomicIsize::new(0),
                sched_entity: self.sched_entity.data_clone(),
            });
            task
        } else {
            // fork as a new process
            let new_tid = tid_alloc();
            debug!("fork new process, tid: {}", new_tid.0);
            let task = Arc::new(Self {
                tgid: Arc::new(AtomicUsize::new(new_tid.0)),
                tid: new_tid,
                pcb: Arc::new(SpinMutex::new(ProcessInfo {
                    children: Vec::new(),
                    parent: Some(Arc::downgrade(self)),
                })),
                memory_set,
                thread: SyncUnsafeCell::new(ThreadInfo {
                    trap_context: self.trap_context().clone(),
                }),
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

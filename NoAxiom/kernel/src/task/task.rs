//! # Task

use alloc::{
    string::ToString,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::sync::atomic::{AtomicIsize, Ordering};

use super::taskid::TidTracer;
use crate::{
    fs::inode::Inode,
    mm::memory_set::MemorySet,
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
    /// task identifier, contains thread_id and process_id
    tid: TidTracer,

    /// task group identifier
    tgid: Arc<usize>,

    /// process control block ptr,
    /// also belongs to other threads
    process: Arc<SpinMutex<ProcessInfo>>,

    /// memory set for task
    /// it's a process resource as well
    memory_set: Arc<SpinMutex<MemorySet>>,

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
    #[inline(always)]
    pub fn tgid(&self) -> usize {
        *self.tgid
    }
    #[inline(always)]
    pub fn is_leader(&self) -> bool {
        self.tid.0 == *self.tgid
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
    #[inline(always)]
    pub fn token(&self) -> usize {
        self.memory_set.lock().token()
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
    pub async fn new_process(path: &str) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let elf_file = Arc::new(Inode::from("initprocess".to_string())); // todo: now is read from static memory
        let elf_memory_info = MemorySet::load_from_elf(elf_file).await;
        let memory_set = elf_memory_info.memory_set;
        let elf_entry = elf_memory_info.elf_entry;
        let user_sp = elf_memory_info.user_sp;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid = tid_alloc();
        let tgid = Arc::new(tid.0);
        // create task
        let task = Arc::new(Self {
            tid,
            tgid,
            process: Arc::new(SpinMutex::new(ProcessInfo {
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
    pub fn fork(&self, flags: CloneFlags) -> Arc<Self> {
        // memory set clone
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set.clone()
        } else {
            Arc::new(SpinMutex::new(self.memory_set.lock().clone_cow()))
        };

        // TODO: fd table
        // let fd = if flags.contains(CloneFlags::FILES) {
        // self.fd_table.clone()
        // } else {
        // self.fd_table.clone_cow()
        // };

        if flags.contains(CloneFlags::THREAD) {
            // fork as a new thread
            let task = Arc::new(Self {
                tid: tid_alloc(),
                tgid: self.tgid.clone(),
                process: self.process.clone(),
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
            todo!()
        }
    }

    /// exit current task
    pub fn exit(&self) {
        self.set_status(TaskStatus::Zombie);
    }
}

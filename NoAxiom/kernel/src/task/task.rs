//! # Task

use alloc::sync::Arc;
use core::sync::atomic::{AtomicI8, AtomicUsize};

use super::taskid::TaskId;
use crate::{
    mm::MemorySet,
    println,
    sched::spawn_task,
    sync::{cell::SyncUnsafeCell, mutex::SpinMutex},
    task::{load_app::get_app_data, taskid::tid_alloc},
    trap::{context::TrapContext, handler::user_trap_handler, trap_return},
};

pub struct ProcessControlBlock {
    pub pid: AtomicUsize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

/// process resources info
pub struct ProcessInfo {
    /// memory set
    pub memory_set: MemorySet,
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
    /// task identifier
    tid: TaskId,

    /// process control block ptr,
    /// also belongs to other threads
    process: Arc<SpinMutex<ProcessInfo>>,

    /// thread control block ptr
    thread: SyncUnsafeCell<ThreadInfo>,

    /// task status: ready / running / zombie
    status: SpinMutex<TaskStatus>,

    /// task exit code
    exit_code: AtomicI8,
}

/// user tasks
/// - usage: wrap it in Arc<Task>
impl Task {
    /// tid
    pub fn tid(&self) -> usize {
        self.tid.0
    }

    /// status
    pub fn set_status(&self, status: TaskStatus) {
        *self.status.lock() = status;
    }
    pub fn is_zombie(&self) -> bool {
        *self.status.lock() == TaskStatus::Zombie
    }
    pub fn is_running(&self) -> bool {
        *self.status.lock() == TaskStatus::Running
    }
    pub fn is_ready(&self) -> bool {
        *self.status.lock() == TaskStatus::Ready
    }

    /// exit code
    pub fn exit_code(&self) -> i8 {
        self.exit_code.load(core::sync::atomic::Ordering::Relaxed)
    }
    pub fn set_exit_code(&self, exit_code: i8) {
        self.exit_code
            .store(exit_code, core::sync::atomic::Ordering::Relaxed);
    }

    /// thread info
    pub fn thread(&self) -> &ThreadInfo {
        unsafe { &(*self.thread.get()) }
    }
    pub fn thread_mut(&self) -> &mut ThreadInfo {
        unsafe { &mut (*self.thread.get()) }
    }

    /// memory set
    pub fn set_memory_set(&self, memory_set: MemorySet) {
        self.process.lock().memory_set = memory_set;
    }
    pub fn token(&self) {
        self.process.lock().memory_set.token();
    }

    /// trap context
    pub fn trap_context(&self) -> &TrapContext {
        &self.thread().trap_context
    }
    pub fn trap_context_mut(&self) -> &mut TrapContext {
        &mut self.thread_mut().trap_context
    }
    pub fn set_trap_context(&self, trap_context: TrapContext) {
        self.thread_mut().trap_context = trap_context;
    }
}

/// user task main
pub async fn task_main(task: Arc<Task>) {
    while !task.is_zombie() {
        // kernel -> user
        trap_return(&task);
        if task.is_zombie() {
            break;
        }
        // user -> kernel
        user_trap_handler(&task).await;
    }
}

/// create new process from elf
pub fn spawn_new_process(app_id: usize) {
    info!("[kernel] spawn new process from elf");
    let elf_data = get_app_data(app_id);
    let (memory_set, user_sp, elf_entry) = MemorySet::from_elf(elf_data);
    info!("[kernel] success to load elf data");
    let task = Arc::new(Task {
        tid: tid_alloc(),
        process: Arc::new(SpinMutex::new(ProcessInfo { memory_set })),
        thread: SyncUnsafeCell::new(ThreadInfo {
            trap_context: TrapContext::app_init_cx(elf_entry, user_sp),
        }),
        status: SpinMutex::new(TaskStatus::Ready),
        exit_code: AtomicI8::new(0),
    });
    info!("create a new task, tid {}", task.tid.0);
    spawn_task(task);
}

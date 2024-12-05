//! # Task

use alloc::sync::Arc;
use core::sync::atomic::{AtomicIsize, Ordering};

use super::taskid::TidTracer;
use crate::{
    cpu::hartid,
    fs::get_app_elf,
    mm::memory_set::MemorySet,
    sched::{spawn_task, task_counter::task_count_dec},
    sync::{cell::SyncUnsafeCell, mutex::SpinMutex},
    task::taskid::tid_alloc,
    trap::{trap_restore, user_trap_handler, TrapContext},
};

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
    tid: TidTracer,

    /// process control block ptr,
    /// also belongs to other threads
    process: Arc<SpinMutex<ProcessInfo>>,

    /// thread control block ptr
    thread: SyncUnsafeCell<ThreadInfo>,

    /// task status: ready / running / zombie
    status: SyncUnsafeCell<TaskStatus>,

    /// priority for schedule
    pub prio: Arc<SyncUnsafeCell<isize>>,

    tmp1: usize,
    tmp2: usize,
    tmp3: usize,

    /// task exit code
    exit_code: AtomicIsize,
}

/// user tasks
/// - usage: wrap it in Arc<Task>
#[allow(unused)]
impl Task {
    /// tid
    pub fn tid(&self) -> usize {
        self.tid.0
    }

    /// status
    pub fn status(&self) -> &TaskStatus {
        unsafe { &(*self.status.get()) }
    }
    pub fn status_mut(&self) -> &mut TaskStatus {
        unsafe { &mut (*self.status.get()) }
    }
    pub fn set_status(&self, status: TaskStatus) {
        *self.status_mut() = status;
    }
    pub fn is_zombie(&self) -> bool {
        *self.status() == TaskStatus::Zombie
    }
    pub fn is_running(&self) -> bool {
        *self.status() == TaskStatus::Running
    }
    pub fn is_ready(&self) -> bool {
        *self.status() == TaskStatus::Ready
    }

    /// exit code
    pub fn exit_code(&self) -> isize {
        self.exit_code.load(Ordering::Relaxed)
    }
    pub fn set_exit_code(&self, exit_code: isize) {
        self.exit_code.store(exit_code, Ordering::Relaxed);
    }

    /// prio
    pub fn prio(&self) -> &isize {
        unsafe { &(*self.prio.get()) }
    }
    pub fn set_prio(&self, prio: isize) {
        unsafe { *self.prio.get() = prio };
    }
    pub fn inc_prio(&self) {
        self._debug_prio("inc_prio");
        unsafe { *self.prio.get() += 1 };
    }
    pub fn _debug_prio(&self, msg: &str) {
        let mut ptr = self.exit_code() as *const usize;
        let mut cnt = 0;
        // while (unsafe { *ptr } != 0 && cnt < 10) {
        //     ptr = unsafe { *ptr } as *const usize;
        //     cnt += 1;
        //     debug!("{:#x}", ptr as usize);
        // }
        debug!(
            "hart: {}, tid: {}, prio: {:#x}, exit_code: {:#x}, *trap_cx: {:#x}, msg: {}",
            hartid(),
            self.tid(),
            *self.prio(),
            self.exit_code(), // 0xffffffc080299bc0
            (self.trap_context() as *const TrapContext) as usize,
            // unsafe { *(self.exit_code() as *const usize) },
            msg,
        );
    }

    /// thread info
    pub fn thread(&self) -> &ThreadInfo {
        unsafe { &(*self.thread.get()) }
    }
    pub fn thread_mut(&self) -> &mut ThreadInfo {
        unsafe { &mut (*self.thread.get()) }
    }

    /// process info
    pub fn process(&self) -> &SpinMutex<ProcessInfo> {
        &self.process
    }

    /// memory set
    pub unsafe fn memory_activate(&self) {
        unsafe { self.process.lock().memory_set.activate() };
    }
    pub fn token(&self) -> usize {
        self.process.lock().memory_set.token()
    }
    pub fn set_memory_set(&self, memory_set: MemorySet) {
        self.process.lock().memory_set = memory_set;
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
    task._debug_prio("task_main begin");
    while !task.is_zombie() {
        // kernel -> user
        task._debug_prio("task_main trap_restore begin1");
        info!("[task_main] trap_restore");
        trap_restore(&task);
        task._debug_prio("task_main trap_restore end");
        if task.is_zombie() {
            warn!("task {} is zombie, break", task.tid());
            break;
        }
        // user -> kernel
        info!("[task_main] user_trap_handler");
        user_trap_handler(&task).await;
    }
    task._debug_prio("task_main end");
    task_count_dec();
}

/// create new process from elf
pub async fn spawn_new_process(app_id: usize) {
    info!("[kernel] spawn new process from elf");
    let elf_file = Arc::new(get_app_elf(app_id)); // todo: now is read from static memory
    let elf_memory_info = MemorySet::load_from_elf(elf_file).await;
    let memory_set = elf_memory_info.memory_set;
    let elf_entry = elf_memory_info.elf_entry;
    let user_sp = elf_memory_info.user_sp;
    info!("[kernel] succeed to load elf data");

    let task = Arc::new(Task {
        tid: tid_alloc(),
        process: Arc::new(SpinMutex::new(ProcessInfo {
            // pid: pid_alloc(), // TODO: pid_alloc()
            memory_set,
        })),
        thread: SyncUnsafeCell::new(ThreadInfo {
            trap_context: TrapContext::app_init_cx(elf_entry, user_sp),
        }),
        status: SyncUnsafeCell::new(TaskStatus::Ready),
        exit_code: AtomicIsize::new(114514),
        tmp1: 0,
        tmp2: 0,
        tmp3: 0,
        prio: Arc::new(SyncUnsafeCell::new(0)),
    });
    debug!(
        "[spawn] prio: {}, exit_code: {}",
        task.prio(),
        task.exit_code()
    );
    info!("[spawn] create a new task, tid {}", task.tid.0);
    spawn_task(task);
}

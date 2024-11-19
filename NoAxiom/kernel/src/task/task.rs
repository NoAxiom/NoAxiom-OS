//! # Task

use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use core::{
    sync::atomic::{AtomicI8, AtomicUsize},
    task::Waker,
};

use super::taskid::TaskId;
use crate::{
    mm::MemorySet,
    println,
    sched::spawn_utask,
    sync::mutex::SpinMutex,
    task::{load_app::get_app_data, taskid::tid_alloc},
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

/// Task Control Block
/// 使用了细粒度的Arc进行锁定, 而不是使用大锁+inner进行锁定
pub struct Task {
    /// task id
    pub tid: TaskId,

    // only for temporary debug
    pub debug_message: String,
    // task status: ready / running / zombie
    // pub status: SpinMutex<TaskStatus>,

    // task exit code
    // pub exit_code: AtomicI8,

    // async waker
    // TODO: consider move to other struct
    // pub waker: Option<Waker>,
}

impl Task {
    // status
    // pub fn set_status(&self, status: TaskStatus) {
    //     *self.status.lock() = status;
    // }
    // pub fn is_zombie(&self) -> bool {
    //     *self.status.lock() == TaskStatus::Zombie
    // }
    // pub fn is_running(&self) -> bool {
    //     *self.status.lock() == TaskStatus::Running
    // }
    // pub fn is_ready(&self) -> bool {
    //     *self.status.lock() == TaskStatus::Ready
    // }

    // exit code
    // pub fn exit_code(&self) -> i8 {
    //     self.exit_code.load(core::sync::atomic::Ordering::Relaxed)
    // }
    // pub fn set_exit_code(&self, exit_code: i8) {
    //     self.exit_code
    //         .store(exit_code, core::sync::atomic::Ordering::Relaxed);
    // }

    // debug message
    pub fn set_debug_message(&mut self, message: String) {
        self.debug_message = message;
    }
    pub fn test(&self) {
        println!(
            "[test] Task is running, Debug message: {}",
            self.debug_message
        );
    }

    /// 通过 elf 数据新建一个任务控制块，
    pub async fn new(app_id: usize) {
        let elf_data = get_app_data(app_id);
        // println!("elf_data: {:?}", elf_data);
        // 解析传入的 ELF 格式数据构造应用的地址空间 memory_set 并获得其他信息
        let (memory_set, user_sp, elf_entry) = MemorySet::from_elf(elf_data);
        log::info!("success to load elf data");
        let taskid = tid_alloc();
        let task = Arc::new(Self {
            tid: taskid,
            debug_message: "CRATE new task".to_string(),
            // task_status: SpinMutex::new(TaskStatus::Ready),
            // pgid: AtomicUsize::new(0),
            // tgid: AtomicUsize::new(tgid),
            // pending_signals: Arc::new(SpinMutex::new(PendingSigs::new())),
            // sigactions: Arc::new(SpinMutex::new(SigActions::new())),
            // memory_set: Arc::new(SpinMutex::new(memory_set)),
            // fd_table: Arc::new(SpinMutex::new(FdTable::new())),
            // thread_group: Arc::new(SpinMutex::new(ThreadGroup::new())),
            // futex_queue: Arc::new(SpinMutex::new(FutexQueue::new())),
            // pcb: Arc::new(SpinMutex::new(ProcessInfo {
            //     trap_cause: None,
            //     parent: None,
            //     children: Vec::new(),
            //     // thread_group: ThreadGroup::new(),
            //     // exit_code: None,
            //     current_path: AbsolutePath::from_str("/"), // cwd
            //     interval_timer: None,
            //     rlimit_nofile: RLimit::new(FD_LIMIT, FD_LIMIT),
            //     robust_list: RobustList::default(),
            //     // futex_queue: FutexQueue::new(),
            //     // #[cfg(not(feature = "multicore"))]
            //     // pselect_times: 0,
            // })),
            // threadinfo: SyncUnsafeCell::new(ThreadInfo {
            //     trap_context: {
            //         let trap_cx = TrapContext::app_init_context(entry_point, user_sp);
            //         trap_cx
            //     },
            //     cpu_mask: CpuMask::new(),
            //     waker: None,
            //     clear_child_tid: None,
            //     set_child_tid: None,
            //     timeinfo: TimeInfo::new(),
            // }),
            // exit_signal: AtomicU8::new(0),
            // sig_struct: SyncUnsafeCell::new(SignalStruct::new()),
            // interrupt_count: SpinMutex::new(BTreeMap::new()),
            // exitcode: AtomicI32::new(0),
        });
        log::info!("create a new task, tid {}", task.tid.0);
        spawn_utask(task);
    }
}

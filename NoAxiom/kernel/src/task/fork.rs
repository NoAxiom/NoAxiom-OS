use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use arch::{Arch, ArchMemory};
use include::errno::{Errno, SyscallResult};

use crate::{
    include::process::CloneFlags,
    mm::user_ptr::UserPtr,
    sched::{
        sched_entity::SchedEntity,
        spawn::spawn_utask,
        utils::{suspend_now, take_waker},
    },
    task::{
        context::TaskTrapContext,
        futex::FutexQueue,
        manager::{ThreadGroup, PROCESS_GROUP_MANAGER, TASK_MANAGER},
        pcb::PCB,
        task::{Mutable, Shared, ThreadOnly},
        taskid::tid_alloc,
        tcb::TCB,
        Task,
    },
    time::timer::ITimerManager,
};

impl Task {
    /// clone current task
    pub async fn do_fork(
        self: &Arc<Self>,
        flags: usize,
        stack: usize,
        ptid: usize,
        tls: usize,
        ctid: usize,
    ) -> SyscallResult {
        let flags = CloneFlags::from_bits(flags & !0xff).ok_or(Errno::EINVAL)?;
        let new_task = self.inner_fork(flags);
        let new_tid = new_task.tid();
        let new_cx = new_task.trap_context_mut();
        debug!(
            "[sys_fork] flags: {:?} stack: {:#x} ptid: {:#x} tls: {:#x} ctid: {:#x}",
            flags, stack, ptid, tls, ctid
        );
        use arch::TrapArgs::*;
        if stack != 0 {
            new_cx[SP] = stack;
        }
        if flags.contains(CloneFlags::SETTLS) {
            new_cx[TLS] = tls;
        }
        if flags.contains(CloneFlags::PARENT_SETTID) {
            let ptid = UserPtr::<usize>::new(ptid);
            ptid.write(new_tid).await?;
        }
        if flags.contains(CloneFlags::CHILD_SETTID) {
            new_task.tcb_mut().set_child_tid = Some(ctid);
        }
        if flags.contains(CloneFlags::CHILD_CLEARTID) {
            new_task.tcb_mut().clear_child_tid = Some(ctid);
        }
        new_cx[RES] = 0;
        trace!("[sys_fork] new task context: {:?}", new_cx);
        info!(
            "[sys_fork] parent: TID{} child: TID{}",
            self.tid(),
            new_task.tid(),
        );
        let has_vfork = flags.contains(CloneFlags::VFORK);
        if has_vfork {
            let waker = take_waker().await;
            new_task.register_vfork_info(waker);
        }
        spawn_utask(&new_task);
        if has_vfork {
            if let Some((vfork_flag, _)) = new_task.vfork_flag() {
                self.vfork_wait_for_completion(vfork_flag).await;
            }
        }
        // TASK_MANAGER.get_init_proc().print_child_tree();
        Ok(new_tid as isize)
    }

    fn inner_fork(self: &Arc<Task>, flags: CloneFlags) -> Arc<Self> {
        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            let new_memory_set = self.memory_set().lock().clone_cow();
            Arch::tlb_flush();
            Shared::new(new_memory_set)
        };

        let sa_list = if flags.contains(CloneFlags::SIGHAND) {
            self.sa_list.clone()
        } else {
            Shared::new(self.sa_list.lock().clone())
        };

        let fd_table = if flags.contains(CloneFlags::FILES) {
            self.fd_table.clone()
        } else {
            Shared::new(self.fd_table.lock().clone())
        };

        // CLONE_PARENT (since Linux 2.3.12)
        //   If CLONE_PARENT is set, then the parent of the new child
        //   (as returned by getppid(2)) will be the same as that of the
        //   calling process.
        // If CLONE_PARENT is not set, then (as with fork(2)) the
        // child's parent is the calling process.
        let parent = if flags.contains(CloneFlags::PARENT) {
            self.pcb.lock().parent.clone()
        } else {
            Some(Arc::downgrade(self))
        };

        let sup_groups = Mutable::new(self.sup_groups().clone());

        let res = if flags.contains(CloneFlags::THREAD) {
            // fork as a new thread
            let new_tid = tid_alloc();
            let tid_val = new_tid.0;
            info!("fork new thread, tid: {}", tid_val);
            let new_thread = Arc::new(Self {
                tid: new_tid,
                tgid: self.tgid.clone(),
                pgid: self.pgid.clone(),
                thread_group: self.thread_group.clone(),
                pcb: Mutable::new(PCB {
                    parent,
                    ..Default::default()
                }),
                memory_set: ThreadOnly::new(memory_set),
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                dir_cwd: self.dir_cwd.clone(),
                dir_exe: self.dir_exe.clone(),
                dir_root: self.dir_root.clone(),
                dir_proc: Shared::new(Self::set_dir_proc(tid_val)),
                sa_list,
                tcb: ThreadOnly::new(TCB {
                    cx: TaskTrapContext::new(self.trap_context().clone(), true),
                    ..Default::default()
                }),
                futex: self.futex.clone(),
                itimer: self.itimer.clone(),
                uid: AtomicU32::new(self.uid()),
                gid: AtomicU32::new(self.gid()),
                fsuid: AtomicU32::new(self.fsuid()),
                fsgid: AtomicU32::new(self.fsgid()),
                euid: AtomicU32::new(self.euid()),
                egid: AtomicU32::new(self.egid()),
                suid: AtomicU32::new(self.suid()),
                sgid: AtomicU32::new(self.sgid()),
                sup_groups,
            });
            new_thread.thread_group.lock().insert(&new_thread);
            TASK_MANAGER.insert(&new_thread);
            new_thread
        } else {
            // fork as a new process
            let new_tid_tracer = tid_alloc();
            let new_tid = new_tid_tracer.0;
            let new_pgid = self.get_pgid(); // use parent's pgid
            info!("fork new process, tgid: {}", new_tid);
            let new_process = Arc::new(Self {
                tid: new_tid_tracer,
                tgid: new_tid,
                pgid: Shared::new_atomic(new_pgid),
                thread_group: Shared::new(ThreadGroup::new()),
                pcb: Mutable::new(PCB {
                    parent,
                    ..Default::default()
                }),
                memory_set: ThreadOnly::new(memory_set),
                sched_entity: ThreadOnly::new(SchedEntity::default()),
                fd_table,
                dir_cwd: Shared::new(self.cwd().clone()),
                dir_exe: Shared::new(self.exe().clone()),
                dir_root: Shared::new(self.root().clone()),
                dir_proc: Shared::new(Self::set_dir_proc(new_tid)),
                sa_list,
                tcb: ThreadOnly::new(TCB {
                    cx: TaskTrapContext::new(self.trap_context().clone(), true),
                    ..Default::default()
                }),
                futex: Shared::new(FutexQueue::new()),
                itimer: Shared::new(ITimerManager::new()),
                uid: AtomicU32::new(self.uid()),
                gid: AtomicU32::new(self.gid()),
                fsuid: AtomicU32::new(self.fsuid()),
                fsgid: AtomicU32::new(self.fsgid()),
                euid: AtomicU32::new(self.euid()),
                egid: AtomicU32::new(self.egid()),
                suid: AtomicU32::new(self.suid()),
                sgid: AtomicU32::new(self.sgid()),
                sup_groups,
            });
            new_process.thread_group().insert(&new_process);
            self.pcb().children.push(new_process.clone());
            TASK_MANAGER.insert(&new_process);
            PROCESS_GROUP_MANAGER.lock().insert(&new_process);
            new_process
        };
        res
    }

    /// wait child to exit or execve for CloneFlags::VFORK
    pub async fn vfork_wait_for_completion(&self, flag: Arc<AtomicBool>) {
        while flag.load(Ordering::SeqCst) {
            suspend_now().await;
        }
    }

    /// callback for vfork, wake suspended parent
    pub fn vfork_callback(&self) {
        if let Some((flag, waker)) = self.tcb_mut().vfork_wait.take() {
            flag.store(true, Ordering::SeqCst);
            waker.wake();
        }
    }
}

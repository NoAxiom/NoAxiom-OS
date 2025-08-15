use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use arch::{Arch, ArchMemory};
use config::mm::PAGE_SIZE;
use include::{
    errno::{Errno, SysResult, SyscallResult},
    return_errno,
};

use crate::{
    include::process::{CloneArgs, CloneFlags},
    mm::user_ptr::UserPtr,
    sched::{
        sched_entity::SchedEntity,
        spawn::spawn_utask,
        utils::{suspend_now, take_waker},
    },
    signal::signal::Signal,
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
    pub async fn do_fork(self: &Arc<Self>, args: CloneArgs) -> SyscallResult {
        let flags = args.flags as usize;
        let flags = CloneFlags::from_bits(flags & !0xff).ok_or(Errno::EINVAL)?;
        let has_stack = args.stack != 0;
        // debug!("[sys_fork] args: {:#x?}", args);
        if has_stack && args.stack_size == 0 {
            return Err(Errno::EINVAL);
        }
        if args.stack_size & 8 as u64 != 0 {
            return_errno!(Errno::EINVAL);
        }
        if flags.contains(CloneFlags::THREAD) && !flags.contains(CloneFlags::SIGHAND) {
            return Err(Errno::EINVAL);
        }
        let exit_signal = if args.exit_signal != 0 {
            Some(Signal::from_repr(args.exit_signal as usize).ok_or(Errno::EINVAL)?)
        } else {
            None
        };
        let new_task = self.inner_fork(flags)?;
        new_task.tcb_mut().exit_signal = exit_signal;
        let new_tid = new_task.tid();
        let new_cx = new_task.trap_context_mut();
        info!("[sys_fork] flags: {:?}, args: {:#x?}", flags, args);
        use arch::TrapArgs::*;
        if has_stack {
            new_cx[SP] = args.stack as usize;
        }
        if flags.contains(CloneFlags::SETTLS) {
            new_cx[TLS] = args.tls as usize;
        }
        if flags.contains(CloneFlags::PARENT_SETTID) {
            let ptid = UserPtr::<usize>::new(args.parent_tid as usize);
            ptid.write(new_tid).await?;
        }
        let ctid = args.child_tid as usize;
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

    fn inner_fork(self: &Arc<Task>, flags: CloneFlags) -> SysResult<Arc<Self>> {
        let sa_list = if flags.contains(CloneFlags::SIGHAND) {
            if !flags.contains(CloneFlags::VM) {
                return Err(Errno::EINVAL);
            }
            self.sa_list.clone()
        } else {
            Shared::new(self.sa_list.lock().clone())
        };

        let memory_set = if flags.contains(CloneFlags::VM) {
            self.memory_set().clone()
        } else {
            let new_memory_set = self.memory_set().lock().clone_cow();
            Arch::tlb_flush();
            Shared::new(new_memory_set)
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
                sched_entity: ThreadOnly::new(SchedEntity::new(self.sched_entity().nice)),
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
                user_id: Mutable::new(self.user_id.lock().clone()),
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
            let new_cwd = self.cwd().clone();
            let new_exe = self.exe().clone();
            let new_root = self.root().clone();
            info!("fork new process, tgid: {}", new_tid);
            assert_no_lock!();
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
                sched_entity: ThreadOnly::new(SchedEntity::new(self.sched_entity().nice)),
                fd_table,
                dir_cwd: Shared::new(new_cwd),
                dir_exe: Shared::new(new_exe),
                dir_root: Shared::new(new_root),
                dir_proc: Shared::new(Self::set_dir_proc(new_tid)),
                sa_list,
                tcb: ThreadOnly::new(TCB {
                    cx: TaskTrapContext::new(self.trap_context().clone(), true),
                    ..Default::default()
                }),
                futex: Shared::new(FutexQueue::new()),
                itimer: Shared::new(ITimerManager::new()),
                user_id: Mutable::new(self.user_id.lock().clone()),
                sup_groups,
            });
            new_process.thread_group().insert(&new_process);
            self.pcb().children.push(new_process.clone());
            TASK_MANAGER.insert(&new_process);
            PROCESS_GROUP_MANAGER.lock().insert(&new_process);
            new_process
        };
        Ok(res)
    }

    /// wait child to exit or execve for CloneFlags::VFORK
    pub async fn vfork_wait_for_completion(&self, flag: Arc<AtomicBool>) {
        while !flag.load(Ordering::SeqCst) {
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

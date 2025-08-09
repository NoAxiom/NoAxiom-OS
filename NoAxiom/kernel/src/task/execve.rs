use alloc::{string::String, sync::Arc, vec::Vec};
use core::{ptr::null, sync::atomic::AtomicU32};

use arch::{Arch, ArchInfo, ArchTrapContext, TrapContext};
use include::errno::SysResult;

use crate::{
    entry::init_proc::INIT_PROC_NAME,
    fs::{
        fdtable::FdTable,
        vfs::{
            basic::{dentry::Dentry, file::File},
            root_dentry,
        },
    },
    include::process::auxv::{AuxEntry, AT_NULL, AT_RANDOM},
    mm::memory_set::{ElfMemoryInfo, MemorySet},
    sched::sched_entity::SchedEntity,
    signal::{sig_action::SigActionList, sig_manager::SigManager, sig_set::SigSet},
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
    /// create new init process from elf
    pub async fn new_init_process(elf: ElfMemoryInfo) -> Arc<Self> {
        trace!("[kernel] spawn new process from elf");
        let ElfMemoryInfo {
            memory_set,
            entry_point: elf_entry,
            user_sp,
            auxs: _,
        } = elf;
        let user_sp = user_sp - 16;
        trace!("[kernel] succeed to load elf data");
        // identifier
        let tid_tracer = tid_alloc();
        let tid = tid_tracer.0;
        // create task
        let task = Arc::new(Self {
            tid: tid_tracer,
            tgid: tid,
            pgid: Shared::new_atomic(tid),
            pcb: Mutable::new(PCB {
                signals: SigManager::default(),
                ..Default::default()
            }),
            thread_group: Shared::new(ThreadGroup::new()),
            memory_set: ThreadOnly::new(Shared::new(memory_set)),
            sched_entity: ThreadOnly::new(SchedEntity::default()),
            fd_table: Shared::new(FdTable::new()),
            dir_cwd: Shared::new(root_dentry()),
            dir_exe: Shared::new(format!("/{}", INIT_PROC_NAME)), // executable path
            dir_root: Shared::new(root_dentry()),
            dir_proc: Shared::new(Self::set_dir_proc(tid)),
            sa_list: Shared::new(SigActionList::new()),
            tcb: ThreadOnly::new(TCB {
                cx: TaskTrapContext::new(TrapContext::app_init_cx(elf_entry, user_sp), true),
                sig_mask: SigSet::all(),
                ..Default::default()
            }),
            futex: Shared::new(FutexQueue::new()),
            itimer: Shared::new(ITimerManager::new()),
            uid: AtomicU32::new(0),               // default user id
            gid: AtomicU32::new(0),               // default group id
            fsuid: AtomicU32::new(0),             // default fs user id
            fsgid: AtomicU32::new(0),             // default fs group id
            euid: AtomicU32::new(0),              // default effective user id
            egid: AtomicU32::new(0),              // default effective group id
            suid: AtomicU32::new(0),              // default saved user id
            sgid: AtomicU32::new(0),              // default saved group id
            sup_groups: Mutable::new(Vec::new()), // default supplementary groups
        });
        task.thread_group().insert(&task);
        TASK_MANAGER.insert(&task);
        PROCESS_GROUP_MANAGER.lock().insert(&task);
        info!("[spawn] new task spawn complete, tid {}", task.tid.0);
        task
    }

    /// initialize the process's proc/self path
    pub fn set_dir_proc(_tid: usize) -> Arc<dyn Dentry> {
        // todo: unimplmented
        root_dentry()
    }

    /// init user stack with pushing arg, env, and auxv
    pub fn init_user_stack(
        &self,
        mut user_sp: usize,
        args: Vec<String>,        // argv & argc
        envs: Vec<String>,        // env vec
        auxs: &mut Vec<AuxEntry>, // aux vec
    ) -> (usize, usize, usize, usize) {
        /// push a data slice with alignment
        /// this func will update user_sp
        fn push_slice<T: Copy>(user_sp: &mut usize, slice: &[T]) {
            let mut sp = *user_sp;
            sp -= core::mem::size_of_val(slice);
            sp -= sp % core::mem::align_of::<T>();
            unsafe { core::slice::from_raw_parts_mut(sp as *mut T, slice.len()) }
                .copy_from_slice(slice);
            *user_sp = sp
        }
        /// align sp with 16 bytes (usize*2)
        macro_rules! align_16 {
            ($sp:ident) => {
                $sp = $sp & !0xf;
            };
        }

        // argv, envp are vectors of each arg's/env's addr
        let mut argv = vec![0; args.len()];
        let mut envp = vec![0; envs.len()];

        // copy each env to the newly allocated stack
        for i in 0..envs.len() {
            // here we leave one byte to store a '\0' as a terminator
            user_sp -= envs[i].len() + 1;
            let p: *mut u8 = user_sp as *mut u8;
            unsafe {
                envp[i] = user_sp;
                p.copy_from(envs[i].as_ptr(), envs[i].len());
                *((p as usize + envs[i].len()) as *mut u8) = 0;
            }
        }
        align_16!(user_sp);

        // copy each arg to the newly allocated stack
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            let p = user_sp as *mut u8;
            unsafe {
                argv[i] = user_sp;
                p.copy_from(args[i].as_ptr(), args[i].len());
                *((p as usize + args[i].len()) as *mut u8) = 0;
            }
        }
        align_16!(user_sp);

        // copy platform
        let platform = Arch::ARCH_NAME;
        user_sp -= platform.len() + 1;
        align_16!(user_sp);
        let p = user_sp as *mut u8;
        unsafe {
            p.copy_from(platform.as_ptr(), platform.len());
            *((p as usize + platform.len()) as *mut u8) = 0;
        }

        // copy 16 random bytes (here is 0)
        user_sp -= 16;
        auxs.push(AuxEntry(AT_RANDOM, user_sp as usize));
        auxs.push(AuxEntry(AT_NULL, 0 as usize));
        align_16!(user_sp);

        // construct auxv
        let len = auxs.len() * core::mem::size_of::<AuxEntry>();
        user_sp -= len;
        for i in 0..auxs.len() {
            unsafe {
                *((user_sp + i * core::mem::size_of::<AuxEntry>()) as *mut usize) = auxs[i].0;
                *((user_sp + i * core::mem::size_of::<AuxEntry>() + core::mem::size_of::<usize>())
                    as *mut usize) = auxs[i].1;
            }
        }

        // construct envp
        let len = (envs.len() + 1) * core::mem::size_of::<usize>();
        user_sp -= len;
        let envp_base = user_sp;
        for i in 0..envs.len() {
            unsafe { *((envp_base + i * core::mem::size_of::<usize>()) as *mut usize) = envp[i] };
        }
        unsafe { *((envp_base + envs.len() * core::mem::size_of::<usize>()) as *mut usize) = 0 };

        // push argv, argc
        push_slice(&mut user_sp, &[null::<u8>()]);
        push_slice(&mut user_sp, argv.as_slice());
        let argv_base = user_sp;
        push_slice(&mut user_sp, &[args.len()]);
        (user_sp, args.len(), argv_base, envp_base)
    }

    /// execute
    pub async fn execve(
        self: &Arc<Self>,
        elf_file: Arc<dyn File>,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> SysResult<()> {
        let ElfMemoryInfo {
            memory_set,
            entry_point,
            user_sp,
            mut auxs,
        } = MemorySet::load_elf(&elf_file).await?;
        memory_set.memory_activate();
        *self.exe() = elf_file.path();
        self.terminate_threads();
        self.change_memory_set(memory_set);
        let (user_sp, _argc, _argv_base, _envp_base) =
            self.init_user_stack(user_sp, args, envs, &mut auxs);
        *self.trap_context_mut() = TrapContext::app_init_cx(entry_point, user_sp);
        self.sa_list().reset();
        self.fd_table().close_on_exec();
        self.vfork_callback();
        Ok(())
    }
}

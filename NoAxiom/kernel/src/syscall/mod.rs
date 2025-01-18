use alloc::sync::Arc;
use core::result::Result;

use arch::interrupt::{enable_global_interrupt, is_interrupt_enabled};

use crate::{constant::register::*, nix::result::Errno, task::Task, trap::TrapContext};

pub mod fs;
pub mod mm;
pub mod others;
pub mod process;

use crate::constant::syscall::*;

pub type SyscallResult = Result<isize, Errno>;

/// system call tracer for a task
pub struct Syscall<'a> {
    pub task: &'a Arc<Task>,
}

impl<'a> Syscall<'a> {
    pub fn new(task: &'a Arc<Task>) -> Self {
        Self { task }
    }
    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> SyscallResult {
        trace!("syscall id: {}, args: {:?}", id, args);
        // fixme: turn on the interrupt. When call yield_now().await, it must be in the
        // interrupt off state, because it comes from trap_handler, cpu would turn off
        // the interrupt until cpu ertn. And we call yield_now().await to switch to
        // another task, but the whole life time is in the interrupt off state.
        assert!(!is_interrupt_enabled());
        enable_global_interrupt();
        match id {
            // fs
            SYS_READ => self.sys_read().await,
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,

            // process
            SYS_EXIT => self.sys_exit(),
            SYS_CLONE => self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE => self.sys_exec(args[0], args[1], args[2]).await,

            // mm
            SYS_BRK => todo!(),
            SYS_MMAP => todo!(),
            SYS_MUNMAP => todo!(),

            // others
            SYS_TIMES => Self::sys_times(args[0]),
            SYS_SCHED_YIELD => Self::sys_yield().await,

            // unsupported: return -1
            _ => {
                panic!("unsupported syscall id: {}, args: {:?}", id, args);
            }
        }
    }
}

pub async fn syscall(task: &Arc<Task>, cx: &mut TrapContext) -> isize {
    let res = Syscall::new(task)
        .syscall(
            cx.user_reg[A7],
            [
                cx.user_reg[A0],
                cx.user_reg[A1],
                cx.user_reg[A2],
                cx.user_reg[A3],
                cx.user_reg[A4],
                cx.user_reg[A5],
            ],
        )
        .await;
    match res {
        Ok(res) => res,
        Err(errno) => {
            let errno = errno as isize;
            if errno > 0 {
                -errno
            } else {
                errno
            }
        }
    }
}

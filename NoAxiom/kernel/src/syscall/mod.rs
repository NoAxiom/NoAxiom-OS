use alloc::sync::Arc;
use core::task::Waker;

use crate::{constant::register::*, task::Task, trap::TrapContext};

pub mod fs;
pub mod mm;
pub mod others;
pub mod process;

use crate::constant::syscall::*;

/// system call tracer for a task
pub struct Syscall<'a> {
    pub task: &'a Arc<Task>,
    #[allow(unused)]
    pub waker: Option<Waker>, // TODO: maybe we can remove this
}

impl<'a> Syscall<'a> {
    pub fn new(task: &'a Arc<Task>) -> Self {
        Self { task, waker: None }
    }
    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> isize {
        trace!("syscall id: {}, args: {:?}", id, args);
        match id {
            SYS_EXIT => self.sys_exit(),
            SYS_READ => self.sys_read().await,
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,
            SYS_SCHED_YIELD => Self::sys_yield().await,
            SYS_CLONE => self.sys_fork(args[0], args[1], args[2], args[3], args[4]),
            SYS_EXECVE => self.sys_exec().await,
            // SYS_TIMES => self.sys_times(args[0]),
            _ => {
                error!("unsupported syscall id: {}, args: {:?}", id, args);
                self.task.exit();
                -1
            }
        }
    }
}

pub async fn syscall(task: &Arc<Task>, cx: &mut TrapContext) -> isize {
    Syscall::new(task)
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
        .await
}

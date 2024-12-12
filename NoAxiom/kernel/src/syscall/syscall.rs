#![allow(unused)]

use alloc::sync::Arc;
use core::task::Waker;

use crate::{constant::syscall::*, driver::sbi::shutdown, task::Task};

/// system call tracer for a task
pub struct Syscall<'a> {
    pub task: &'a Arc<Task>,
    pub waker: Option<Waker>,
}

impl<'a> Syscall<'a> {
    pub fn new(task: &'a Arc<Task>) -> Self {
        Self { task, waker: None }
    }
    pub fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> isize {
        trace!("syscall id: {}, args: {:?}", id, args);
        match id {
            SYS_EXIT => self.sys_exit(),
            SYS_READ => self.sys_read().await,
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,
            SYS_SCHED_YIELD => {self.sys_yield().await;},
            SYS_SYSTEMSHUTDOWN => shutdown(), // todo: remove this
            _ => panic!("unsupported syscall id: {}, args: {:?}", id, args),
        }
        -1
    }
}

use alloc::sync::Arc;
use core::task::Waker;

use crate::{
    constant::syscall::*,
    cpu::current_cpu,
    print,
    task::{Task, TaskStatus},
};

/// system call tracer for a task
pub struct Syscall {
    task: Arc<Task>,
    waker: Option<Waker>,
}

impl Syscall {
    pub fn new(task: &Arc<Task>) -> Self {
        Self {
            task: task.clone(),
            waker: None,
        }
    }
    pub fn task(&self) -> Arc<Task> {
        self.task.clone()
    }
    pub fn waker(&self) -> Option<Waker> {
        self.waker.clone()
    }
    pub fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    pub async fn syscall(&mut self, id: usize, args: [usize; 6]) -> isize {
        info!("syscall id: {}, args: {:?}", id, args);
        match id {
            SYS_EXIT => self.sys_exit(),
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,
            _ => panic!("unsupported syscall id: {}, args: {:?}", id, args),
        }
        -1
    }

    pub fn sys_exit(&mut self) {
        let tmp = self.task.status_mut();
        *tmp = TaskStatus::Zombie;
        info!("task exited, tid: {}", self.task.tid());
    }

    pub async fn sys_write(&self, _fd: usize, buf: usize, len: usize) {
        assert!(current_cpu().token() == self.task.token());
        info!("sys_write: fd: {}, buf: {:#x}, len: {}", _fd, buf, len);
        let task = current_cpu().task.clone().unwrap();
        unsafe { task.memory_activate() };
        let buf = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let s = core::str::from_utf8(buf).unwrap();
        print!("{}", s);
    }
}

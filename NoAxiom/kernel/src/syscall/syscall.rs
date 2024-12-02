use alloc::sync::Arc;
use core::task::Waker;

use crate::{
    arch::interrupt::enable_visit_user_memory, constant::syscall::*, cpu::current_cpu, println, task::{Task, TaskStatus}
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
    pub async fn syscall(&self, id: usize, args: [usize; 6]) -> isize {
        info!("syscall id: {}, args: {:?}", id, args);
        if id == SYS_EXIT {
            let tmp = self.task.status_mut();
            *tmp = TaskStatus::Zombie;
            println!("task exited, tid: {}, args {:?}", self.task.tid(), args);
        } else {
            self.sys_write(args[0] as usize, args[1] as usize, args[2] as usize)
                .await;
            // println!("print! syscall id: {}", id);
        }
        -1
    }

    pub async fn sys_write(&self, _fd: usize, buf: usize, len: usize) {
        assert!(current_cpu().token() == self.task.token());
        println!("sys_write: fd: {}, buf: {:#x}, len: {}", _fd, buf, len);
        let task = current_cpu().task.clone().unwrap();
        unsafe { task.memory_activate() };
        let buf = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let s = core::str::from_utf8(buf).unwrap();
        println!("{}", s);
    }
}

use alloc::sync::Arc;
use core::task::Waker;

use crate::{
    constant::syscall::*,
    cpu::get_hartid,
    driver::sbi::shutdown,
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
            SYS_READ => self.sys_read().await, // todo
            SYS_WRITE => self.sys_write(args[0], args[1], args[2]).await,
            SYS_SYSTEMSHUTDOWN => shutdown(),
            _ => panic!("unsupported syscall id: {}, args: {:?}", id, args),
        }
        -1
    }

    // todo: complete this
    pub async fn sys_read(&mut self) {
        todo!()
    }

    // todo: add fd
    pub async fn sys_write(&self, _fd: usize, buf: usize, len: usize) {
        info!(
            "sys_write: fd: {}, buf: {:#x}, len: {}, hart: {}",
            _fd,
            buf,
            len,
            get_hartid()
        );
        let buf = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let s = core::str::from_utf8(buf).unwrap();
        print!("{}", s);
    }

    pub fn sys_exit(&mut self) {
        *self.task.status_mut() = TaskStatus::Zombie;
        debug!(
            "task exited, tid: {}, counter: {}",
            self.task.tid(),
            unsafe {
                crate::sched::task_counter::TASK_COUNTER.load(core::sync::atomic::Ordering::SeqCst)
            }
        );
    }
}

//! # Task

use alloc::string::String;
use core::{
    sync::atomic::{AtomicI8, AtomicUsize},
    task::Waker,
};

use super::tid_allocator::TaskId;
use crate::{println, sync::mutex::SpinMutex};

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

    /// only for temporary debug
    pub debug_message: String,

    /// task status: ready / running / zombie
    pub status: SpinMutex<TaskStatus>,

    /// task exit code
    pub exit_code: AtomicI8,

    /// async waker
    /// TODO: consider move to other struct
    pub waker: Option<Waker>,
}

impl Task {
    // status
    pub fn set_status(&self, status: TaskStatus) {
        *self.status.lock() = status;
    }
    pub fn is_zombie(&self) -> bool {
        *self.status.lock() == TaskStatus::Zombie
    }
    pub fn is_running(&self) -> bool {
        *self.status.lock() == TaskStatus::Running
    }
    pub fn is_ready(&self) -> bool {
        *self.status.lock() == TaskStatus::Ready
    }

    // exit code
    pub fn exit_code(&self) -> i8 {
        self.exit_code.load(core::sync::atomic::Ordering::Relaxed)
    }
    pub fn set_exit_code(&self, exit_code: i8) {
        self.exit_code
            .store(exit_code, core::sync::atomic::Ordering::Relaxed);
    }

    // debug message
    pub fn set_debug_message(&mut self, message: String) {
        self.debug_message = message;
    }
    pub fn exec(&self) {
        println!(
            "[debug] Task {} is running, Debug message: {}",
            self.tid.0, self.debug_message
        );
    }

    // waker
    pub fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }
    pub fn waker(&self) -> &Option<Waker> {
        &self.waker
    }
    pub fn wake(&self) {
        let waker = self.waker();
        waker.as_ref().unwrap().wake_by_ref();
    }
}

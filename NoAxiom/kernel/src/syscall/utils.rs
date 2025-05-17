use crate::{cpu::current_task, include::syscall_id::SyscallID};

pub fn current_syscall() -> SyscallID {
    current_task()
        .map(|task| task.tcb().current_syscall)
        .unwrap_or(SyscallID::NO_SYSCALL)
}

pub fn update_current_syscall(syscall_id: SyscallID) {
    let task = current_task().unwrap();
    task.tcb_mut().current_syscall = syscall_id;
}

pub fn clear_current_syscall() {
    let task = current_task().unwrap();
    task.tcb_mut().current_syscall = SyscallID::NO_SYSCALL;
}
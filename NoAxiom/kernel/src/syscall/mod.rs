use alloc::sync::Arc;

use crate::{constant::register::*, task::Task, trap::TrapContext};

mod syscall;

pub async fn syscall(task: &Arc<Task>, cx: &mut TrapContext) -> isize {
    syscall::Syscall::new(task)
        .syscall(
            cx.regs[A7],
            [
                cx.regs[A0],
                cx.regs[A1],
                cx.regs[A2],
                cx.regs[A3],
                cx.regs[A4],
                cx.regs[A5],
            ],
        )
        .await
}

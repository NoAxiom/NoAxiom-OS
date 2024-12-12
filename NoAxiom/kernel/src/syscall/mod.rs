use alloc::sync::Arc;

use crate::{constant::register::*, task::Task, trap::TrapContext};

mod fs;
mod process;
mod syscall;

pub async fn syscall(task: &Arc<Task>, cx: &mut TrapContext) -> isize {
    syscall::Syscall::new(task)
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

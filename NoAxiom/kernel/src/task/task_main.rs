use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{fence, Ordering},
    task::{Context, Poll},
};

use arch::{Arch, ArchInt, ArchTrap, ArchTrapContext, ArchUserFloatContext};
use ksync::mutex::check_no_lock;

use crate::{
    cpu::current_cpu,
    sched::utils::{suspend_now, take_waker},
    task::{status::TaskStatus, Task},
    time::gettime::get_time_us,
    trap::handler::user_trap_handler,
};

pub struct UserTaskFuture<F: Future + Send + 'static> {
    pub task: Arc<Task>,
    pub future: F,
}

impl<F: Future + Send + 'static> UserTaskFuture<F> {
    pub fn new(task: Arc<Task>, future: F) -> Self {
        Self { task, future }
    }
}

impl<F: Future + Send + 'static> Future for UserTaskFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // ===== interrupt disabled =====
        let old = Arch::is_interrupt_enabled();
        Arch::disable_interrupt();

        // ===== before executing task future =====
        let this = unsafe { self.get_unchecked_mut() };
        let task = &this.task;
        let future = &mut this.future;
        let time_in = get_time_us();
        task.tcb_mut().time_stat.record_switch_in();
        current_cpu().set_task(task);
        // ===== before executing task future =====

        // ===== interrupt restore =====
        if old {
            Arch::enable_interrupt();
        }

        let ret = unsafe { Pin::new_unchecked(future).poll(cx) };

        // ===== interrupt disabled =====
        let old = Arch::is_interrupt_enabled();
        if old {
            Arch::disable_interrupt();
        }

        // ===== after executing task future =====
        let time_out = get_time_us();
        task.tcb_mut().time_stat.record_switch_out();
        task.trap_context_mut().freg_mut().yield_task();
        task.sched_entity().update_vruntime(time_out - time_in);
        current_cpu().clear_task();
        // ===== after executing task future =====

        // ===== interrupt restore =====
        if old {
            Arch::enable_interrupt();
        }
        ret
    }
}

/// user task main
/// called by [`UserTaskFuture`]
pub async fn task_main(task: Arc<Task>) {
    task.set_waker(take_waker().await);
    let mut old_mask = None;
    assert!(check_no_lock());
    loop {
        // kernel -> user
        trace!("[task_main] trap_restore, cx: {:#x?}", task.trap_context());
        task.tcb_mut().time_stat.record_trap_in();
        let cx = task.trap_context_mut();
        Arch::trap_restore(cx); // restore context and return to user mode
        let trap_type = Arch::read_trap_type(Some(cx));
        task.tcb_mut().time_stat.record_trap_out();

        // check sigmask and status
        // unsafe {
        //     let bottom = 0x120b32630usize;
        //     let top = 0x120b327a0usize;
        //     let ptr = bottom as *const u8;
        //     static mut LAST_HASH: u64 = 0;
        //     if task
        //         .memory_set()
        //         .lock()
        //         .page_table()
        //         .find_pte(VirtAddr::from(bottom).floor())
        //         .is_some()
        //     {
        //         let slice = core::slice::from_raw_parts(ptr, top - bottom);
        //         let mut res: u64 = 0;
        //         for it in slice {
        //             res = (res * 20040409 + *it as u64) % 998244353;
        //         }
        //         debug!("[test] hash value: {:#x}", res);
        //         if res != LAST_HASH {
        //             debug!("[test] hash value changed: {:#x} -> {:#x}", LAST_HASH,
        // res);             let slice_res: Vec<_> = slice.iter().collect();
        //             debug!("[test] slice: {:?}", slice_res);
        //             LAST_HASH = res;
        //         }
        //     } else {
        //         debug!("[test] not allocated");
        //     }
        // };
        fence(Ordering::SeqCst);
        assert!(check_no_lock());
        let mut pcb = task.pcb();
        if let Some(old_mask) = old_mask.take() {
            trace!("clear sigmask {:?}", pcb.pending_sigs.sig_mask);
            pcb.pending_sigs.sig_mask = old_mask;
        }
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => {
                warn!("[task_main] task is stopped(1), tid: {}", task.tid());
                suspend_now(pcb).await;
            }
            _ => drop(pcb),
        }
        assert!(check_no_lock());

        // user -> kernel, enter the handler
        trace!(
            "[task_main] user_trap_handler, cx: {:#x?}",
            task.trap_context()
        );
        assert!(!Arch::is_interrupt_enabled());
        assert!(check_no_lock());
        user_trap_handler(&task, trap_type).await;

        // check status
        let pcb = task.pcb();
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => {
                warn!("[task_main] task is stopped(2), tid: {}", task.tid());
                suspend_now(pcb).await;
            }
            _ => drop(pcb),
        }
        assert!(check_no_lock());

        // check signal before return to user
        trace!("[task_main] check_signal");
        old_mask = task.check_signal();
        assert!(check_no_lock());
        let pcb = task.pcb();
        match pcb.status() {
            TaskStatus::Terminated => break,
            TaskStatus::Stopped => {
                warn!("[task_main] task is stopped(3), tid: {}", task.tid());
                suspend_now(pcb).await;
            }
            _ => drop(pcb),
        }
        assert!(check_no_lock());
    }
    assert!(check_no_lock());
    task.exit_handler().await;
}

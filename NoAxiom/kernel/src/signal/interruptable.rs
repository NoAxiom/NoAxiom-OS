use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use include::errno::Errno;
use ksync::assert_no_lock;
use pin_project_lite::pin_project;

use super::sig_set::SigSet;
use crate::{
    include::process::TaskFlags, signal::sig_set::SigMask, syscall::SysResult, task::Task,
};

pin_project! {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct IntableFuture<'a, F> {
        task: &'a Arc<Task>,
        #[pin]
        fut: F,
        mask: SigMask,
    }
}

impl<F, T> Future for IntableFuture<'_, F>
where
    F: Future<Output = T>,
{
    type Output = SysResult<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let task = this.task;
        match this.fut.poll(cx) {
            Poll::Ready(res) => Poll::Ready(Ok(res)),
            Poll::Pending => {
                // start to handle signal
                let mask = this.mask;
                let pcb = task.pcb();
                if pcb.signals.has_pending_signals(*mask) {
                    warn!(
                        "[intable] TID{} get interrupted, mask {:?}, pending {:?}",
                        task.tid(),
                        mask.debug_info_short(),
                        pcb.signals.pending_set.debug_info_short()
                    );
                    task.record_current_result_reg();
                    task.tcb_mut().flags |= TaskFlags::TIF_SIGPENDING;
                    Poll::Ready(Err(Errno::EINTR))
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

/// ## Interruptable Future
///
/// This future wrapper provides pending signal check
/// when the inner future returns Pending.
///
/// - `task`: The task that is waiting for the future.
/// - `fut`: The future to be executed.
/// - `new_mask`: The new signal mask to be set for the task, which will be
///   restored in [Task::check_signal].
/// - `wake_set`: The *additional* signal set that will wake the task.
pub async fn interruptable<T>(
    task: &Arc<Task>,
    fut: impl Future<Output = T>,
    new_mask: Option<SigMask>,
    wake_set: Option<SigSet>,
) -> SysResult<T> {
    // ensure the task is not holding any spinlocks
    assert_no_lock!();

    // set new mask
    if let Some(new_mask) = new_mask {
        task.swap_in_sigmask(new_mask.without_kill());
        task.tif_mut().insert(TaskFlags::TIF_RESTORE_SIGMASK);
    }
    let mask = task.sig_mask();

    // set wake signal: forbid masked and ignored signals
    let wake_set = wake_set.unwrap_or(SigSet::empty());
    let ignored_set = task.sa_list().get_ignored_bitmap();
    let wake_set = !(mask | ignored_set) | wake_set;
    task.pcb().set_wake_signal(wake_set);

    // suspend now!
    debug!(
        "[intable] TID{} wait with wake set: {}, mask: {}",
        task.tid(),
        wake_set.debug_info_short(),
        mask.debug_info_short(),
    );
    let res = IntableFuture { task, fut, mask }.await;

    // restore old mask
    task.pcb().clear_wake_signal();
    res
}

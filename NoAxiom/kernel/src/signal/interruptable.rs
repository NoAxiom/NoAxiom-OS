use alloc::sync::Arc;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use include::errno::Errno;
use ksync::assert_no_lock;
use pin_project_lite::pin_project;

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
                if let Some(info) = task.pcb().signals.peek_with_mask(*mask) {
                    warn!(
                        "[intable] TID{} get interrupted by signal {:?}, mask {:?}",
                        task.tid(),
                        info.signal,
                        mask,
                    );
                    task.tcb_mut().flags |= TaskFlags::TIF_SIGPENDING;
                    Poll::Ready(Err(Errno::EINTR))
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub async fn interruptable<T>(
    task: &Arc<Task>,
    fut: impl Future<Output = T>,
    new_mask: Option<SigMask>,
) -> SysResult<T> {
    // ensure the task is not holding any spinlocks
    assert_no_lock!();

    // set wake signal
    // fixme: should we skip ignored sigset?
    let sig_mask = new_mask.unwrap_or(task.sig_mask());
    // let ignored_set = task.sa_list().get_ignored_bitmap();
    // let mask = (sig_mask | ignored_set).without_kill();
    let mask = sig_mask.without_kill();
    task.swap_in_sigmask(mask);
    task.tif_mut().insert(TaskFlags::TIF_RESTORE_SIGMASK);
    task.pcb().set_wake_signal(!mask);

    // suspend now!
    debug!(
        "[intable] TID{} wait with wake set: {:?}",
        task.tid(),
        !mask
    );
    let res = IntableFuture { task, fut, mask }.await;

    // restore old mask
    task.pcb().clear_wake_signal();
    res
}

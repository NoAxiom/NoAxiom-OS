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
    signal::{sig_action::SAFlags, sig_set::SigMask},
    syscall::SysResult,
    task::Task,
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
                    let sa_list = task.sa_list();
                    if sa_list[info.signal].flags.contains(SAFlags::SA_RESTART) {
                        return Poll::Pending;
                    }
                    // task.tcb_mut().interrupted = true;
                    debug!(
                        "[intable] TID{} interrupted by signal {:?}, mask {:?}",
                        task.tid(),
                        info.signal,
                        mask,
                    );
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

    // replace old mask
    let old_mask = task.sig_mask();
    if let Some(mask) = new_mask {
        task.set_sig_mask(mask);
    }
    let sig_mask = task.sig_mask();

    // set wake signal
    let ignored_set = task.sa_list().get_ignored_bitmap();
    let mask = (sig_mask | ignored_set).without_kill();
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
    task.set_sig_mask(old_mask);

    res
}

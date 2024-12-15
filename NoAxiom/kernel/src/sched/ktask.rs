use alloc::sync::Arc;
use core::future::Future;

use crate::{sched::executor::spawn_raw, sync::cell::SyncUnsafeCell};

pub fn schedule_spawn_new_ktask<F, R>(future: F, prio: isize)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    trace!("task_count_inc, counter: {}", unsafe {
        crate::sched::task_counter::TASK_COUNTER.load(core::sync::atomic::Ordering::SeqCst)
    });
    spawn_raw(future, Arc::new(SyncUnsafeCell::new(prio)));
}

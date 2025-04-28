use alloc::sync::Arc;
use core::time::Duration;

use crate::task::Task;

impl Task {
    pub async fn sleep(self: &Arc<Self>, interval: Duration) {
        warn!("called unimplemented sleep: interval = {:?}", interval);
    }
}

pub fn sleep_handler() {}

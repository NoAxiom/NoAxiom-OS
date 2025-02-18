use alloc::sync::Arc;

use super::Task;

pub fn exit_handler(task: &Arc<Task>) {
    warn!("[exit_hander] task {} exited successfully", task.tid());
    // todo: release resources
}

use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    include::process::robust_list::RobustList, mm::user_ptr::UserPtr, return_errno,
    task::manager::TASK_MANAGER,
};

impl Syscall<'_> {
    pub fn sys_set_robust_list(&self, head: usize, len: usize) -> SyscallResult {
        info!("[sys_set_robust_list] head {:#x}, len {:#x}", head, len);
        if len != RobustList::HEAD_SIZE {
            error!("robust list head len mismatch: len={}", len);
            return_errno!(Errno::EINVAL);
        }
        let mut pcb = self.task.pcb();
        pcb.robust_list.head = head;
        pcb.robust_list.len = len;
        Ok(0)
    }

    pub fn sys_get_robust_list(
        &self,
        pid: usize,
        head_ptr: usize,
        len_ptr: usize,
    ) -> SyscallResult {
        warn!("[sys_get_robust_list]");
        info!(
            "[sys_get_robust_list] pid {:?} head {:#x}, len {:#x}",
            pid, head_ptr as usize, len_ptr as usize
        );
        let tid = if pid == 0 { self.task.tid() } else { pid };
        let Some(task) = TASK_MANAGER.get(tid) else {
            return_errno!(Errno::ESRCH);
        };
        let robust_list = task.pcb().robust_list;
        let head_ptr = UserPtr::<usize>::new(head_ptr);
        let len_ptr = UserPtr::<usize>::new(len_ptr);
        head_ptr.write(robust_list.head);
        len_ptr.write(robust_list.len);
        Ok(0)
    }
}

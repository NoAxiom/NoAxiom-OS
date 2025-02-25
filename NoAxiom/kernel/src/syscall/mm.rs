use super::SyscallResult;
use crate::{include::result::Errno, syscall::Syscall};

impl Syscall<'_> {
    pub fn sys_brk(&self, brk: usize) -> SyscallResult {
        let task = self.task;
        if brk == 0 {
            Ok(task.update_brk(0) as isize)
        } else {
            let former_addr = task.update_brk(0);
            let grow_size: isize = (brk - former_addr) as isize;
            Ok(self.task.update_brk(grow_size) as isize)
        }
    }
    pub fn sys_munmap(&self) -> SyscallResult {
        todo!()
        // self.task.munmap();
    }
}

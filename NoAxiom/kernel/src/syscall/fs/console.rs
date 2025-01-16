use crate::{
    cpu::get_hartid,
    mm::user_ptr::UserPtr,
    print,
    syscall::{Syscall, SyscallResult},
};

impl Syscall<'_> {
    // todo: complete this
    pub async fn sys_read(&self) -> SyscallResult {
        todo!()
    }

    // todo: add fd
    pub async fn sys_write(&self, fd: usize, buf: usize, len: usize) -> SyscallResult {
        trace!(
            "sys_write: fd: {}, buf: {:#x}, len: {}, hart: {}",
            fd,
            buf,
            len,
            get_hartid()
        );
        let s = UserPtr::<u8>::new(buf).as_string(len);
        print!("{}", s);
        Ok(0)
    }
}

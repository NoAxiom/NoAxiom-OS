use crate::{cpu::get_hartid, print, syscall::{Syscall, SyscallResult}};

impl Syscall<'_> {
    // todo: complete this
    pub async fn sys_read(&self) -> SyscallResult {
        todo!()
    }

    // todo: add fd
    pub async fn sys_write(&self, _fd: usize, buf: usize, len: usize) -> SyscallResult {
        trace!(
            "sys_write: fd: {}, buf: {:#x}, len: {}, hart: {}",
            _fd,
            buf,
            len,
            get_hartid()
        );
        let buf = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, len) };
        let s = core::str::from_utf8(buf).unwrap();
        print!("{}", s);
        Ok(0)
    }
}

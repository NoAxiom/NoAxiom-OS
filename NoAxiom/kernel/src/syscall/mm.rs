use super::SyscallResult;
use crate::{
    config::mm::PAGE_SIZE,
    include::{
        mm::{MmapFlags, MmapProts},
        result::Errno,
    },
    syscall::Syscall,
    utils::align_up,
};

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

    pub fn sys_mmap(
        &self,
        addr: usize,
        length: usize,
        prot: usize,
        flags: usize,
        fd: isize,
        offset: usize,
    ) -> SyscallResult {
        debug!("[sys_mmap] mmap start");
        let length = align_up(length, PAGE_SIZE);
        let fd_table = self.task.fd_table();
        let prot = MmapProts::from_bits(prot).unwrap();
        let flags = MmapFlags::from_bits(flags).unwrap();
        info!(
            "[sys_mmap]: addr: {:#x}, len: {:#x}, fd: {}, offset: {:#x}, flags: {:?}, prot: {:?}",
            addr, length, fd, offset, flags, prot
        );
        if length == 0 {
            return Err(Errno::EINVAL);
        }
        if !flags.contains(MmapFlags::MAP_ANONYMOUS)
            && (fd as usize >= fd_table.table.len() || fd_table.table[fd as usize].is_none())
        {
            return Err(Errno::EBADF);
        }
        drop(fd_table);
        let result_addr = self.task.mmap(addr, length, prot, flags, fd, offset);
        Ok(result_addr as isize)
    }

    pub fn sys_munmap(&self) -> SyscallResult {
        todo!()
        // self.task.munmap();
    }
}

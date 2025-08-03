use crate::file_default;

file_default!(
    LoopControlFile,
    async fn base_read(&self, _offset: usize, _buf: &mut [u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    },
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
);

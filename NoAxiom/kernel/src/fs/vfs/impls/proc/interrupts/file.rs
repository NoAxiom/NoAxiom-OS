use core::sync::atomic::AtomicUsize;

use crate::file_default;

lazy_static::lazy_static! {
    static ref INTERRUPTS_COUNT: AtomicUsize = AtomicUsize::new(0);
}

pub fn inc_interrupts_count() {
    INTERRUPTS_COUNT.fetch_add(1, core::sync::atomic::Ordering::Release);
}

file_default!(
    InterruptsFile,
    async fn base_read(&self, _offset: usize, buf: &mut [u8]) -> SyscallResult {
        if buf.is_empty() {
            return Ok(0);
        }
        buf[0] = crate::fs::vfs::impls::proc::interrupts::file::INTERRUPTS_COUNT
            .load(core::sync::atomic::Ordering::Relaxed) as u8;
        Ok(1)
    },
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
);

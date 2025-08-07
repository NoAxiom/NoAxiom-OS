use core::sync::atomic::AtomicUsize;

use arch::InterruptNumber;
use array_init::array_init;

use crate::file_default;

const INTERRUPT_NUM: usize = 128;

lazy_static::lazy_static! {
    static ref INTERRUPTS_COUNT: [AtomicUsize; INTERRUPT_NUM] = {
        array_init(|_| AtomicUsize::new(0))
    };
}

/*
5:        8188
8:        1162
10:        397
*/

pub fn inc_interrupts_count(id: InterruptNumber) {
    INTERRUPTS_COUNT[id].fetch_add(1, core::sync::atomic::Ordering::SeqCst);
}

file_default!(
    InterruptsFile,
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        debug!("[Interrupts] offset: {}", offset);

        let mut written = 0;
        let mut read_buf = alloc::vec::Vec::new();
        for (id, counter) in super::INTERRUPTS_COUNT.iter().enumerate() {
            let count = counter.load(core::sync::atomic::Ordering::Relaxed);
            if count == 0 {
                continue;
            }
            let line = alloc::format!("{}: {}\n", id, count);
            debug!(
                "[Interrupts] interrupts: id: {}, count: {}, line: {}",
                id, count, line
            );
            let line_bytes = line.as_bytes();
            read_buf.extend_from_slice(line_bytes);
            written += line_bytes.len();
        }
        if offset >= read_buf.len() {
            return Ok(0);
        }
        let ret_len = written.min(buf.len() - offset);
        buf[offset..offset + ret_len].copy_from_slice(&read_buf[offset..offset + ret_len]);
        Ok(ret_len as isize)
    },
    async fn base_write(&self, _offset: usize, _buf: &[u8]) -> SyscallResult {
        Err(Errno::ENOSYS)
    }
);

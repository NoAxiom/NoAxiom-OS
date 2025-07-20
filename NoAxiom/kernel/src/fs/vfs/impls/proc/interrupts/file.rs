use alloc::collections::btree_map::BTreeMap;
use core::sync::atomic::AtomicUsize;

use ksync::mutex::SpinLock;
use num_traits::abs;

use crate::file_default;

lazy_static::lazy_static! {
    static ref INTERRUPTS_COUNT: SpinLock<BTreeMap<isize, AtomicUsize>> = SpinLock::new(BTreeMap::new());
}

/*
5:        8188
8:        1162
10:        397
*/

pub fn inc_interrupts_count(id: isize) {
    let id = abs(id);
    INTERRUPTS_COUNT
        .lock()
        .entry(id)
        .or_insert_with(|| AtomicUsize::new(0))
        .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
}

file_default!(
    InterruptsFile,
    async fn base_read(&self, offset: usize, buf: &mut [u8]) -> SyscallResult {
        let interrupts = super::INTERRUPTS_COUNT.lock();
        debug!("[Interrupts] offset: {}", offset);

        let mut written = 0;
        let mut read_buf = alloc::vec::Vec::new();
        for (&id, counter) in interrupts.iter() {
            let count = counter.load(core::sync::atomic::Ordering::Relaxed);
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

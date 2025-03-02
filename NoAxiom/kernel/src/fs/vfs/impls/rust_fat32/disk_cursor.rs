use alloc::{boxed::Box, sync::Arc};

use async_trait::async_trait;
use fatfs::*;

use crate::{config::fs::BLOCK_SIZE, device::block::BlockDevice};

#[derive(Clone)]
pub struct DiskCursor {
    blk: Arc<dyn BlockDevice>,
    blk_id: usize,
    offset: usize,
}

impl DiskCursor {
    pub fn new(blk: Arc<dyn BlockDevice>, blk_id: usize, offset: usize) -> Self {
        Self {
            blk,
            blk_id,
            offset,
        }
    }
    fn move_cursor(&mut self, offset: usize) {
        self.set_position(self.position() + offset)
    }
    fn position(&self) -> usize {
        self.blk_id * BLOCK_SIZE + self.offset
    }
    fn set_position(&mut self, pos: usize) {
        self.blk_id = pos / BLOCK_SIZE;
        self.offset = pos % BLOCK_SIZE;
    }
}

impl IoBase for DiskCursor {
    type Error = ();
}

/// The `Read` trait allows for reading bytes from a source.
///
/// It is based on the `std::io::Read` trait.
#[async_trait]
impl Read for DiskCursor {
    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    ///
    /// This function does not provide any guarantees about whether it blocks
    /// waiting for data, but if an object needs to block for a read and
    /// cannot, it will typically signal this via an Err return value.
    ///
    /// If the return value of this method is `Ok(n)`, then it must be
    /// guaranteed that `0 <= n <= buf.len()`. A nonzero `n` value indicates
    /// that the buffer buf has been filled in with n bytes of data from this
    /// source. If `n` is `0`, then it can indicate one of two scenarios:
    ///
    /// 1. This reader has reached its "end of file" and will likely no longer
    ///    be able to produce bytes. Note that this does not mean that the
    ///    reader will always no longer be able to produce bytes.
    /// 2. The buffer specified was 0 bytes in length.
    ///
    /// It is not an error if the returned value `n` is smaller than the buffer
    /// size, even when the reader is not at the end of the stream yet. This
    /// may happen for example because fewer bytes are actually available right
    /// now (e. g. being close to end-of-file) or because `read()` was
    /// interrupted by a signal.
    ///
    /// # Errors
    ///
    /// If this function encounters any form of I/O or other error, an error
    /// will be returned. If an error is returned then it must be guaranteed
    /// that no bytes were read. An error for which
    /// `IoError::is_interrupted` returns true is non-fatal and the read
    /// operation should be retried if there is nothing else to do.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut data = [0; BLOCK_SIZE];
        self.blk.read(self.blk_id, &mut data).await;
        let read_size = (BLOCK_SIZE - self.offset).min(buf.len());
        buf[..read_size].copy_from_slice(&data[self.offset..self.offset + read_size]);
        self.move_cursor(read_size);
        Ok(read_size)
    }
}

/// The `Write` trait allows for writing bytes into the sink.
///
/// It is based on the `std::io::Write` trait.
#[async_trait]
impl Write for DiskCursor {
    /// Write a buffer into this writer, returning how many bytes were written.
    ///
    /// # Errors
    ///
    /// Each call to write may generate an I/O error indicating that the
    /// operation could not be completed. If an error is returned then no
    /// bytes in the buffer were written to this writer.
    /// It is not considered an error if the entire buffer could not be written
    /// to this writer.
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut data = [0; BLOCK_SIZE];
        self.blk.read(self.blk_id as usize, &mut data).await;
        let write_size = (BLOCK_SIZE - self.offset).min(buf.len());
        data[self.offset..self.offset + write_size].copy_from_slice(&buf[..write_size]);
        self.blk.write(self.blk_id as usize, &data).await;
        self.move_cursor(write_size);
        Ok(write_size)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The `Seek` trait provides a cursor which can be moved within a stream of
/// bytes.
///
/// It is based on the `std::io::Seek` trait.
impl Seek for DiskCursor {
    /// Seek to an offset, in bytes, in a stream.
    ///
    /// A seek beyond the end of a stream or to a negative position is not
    /// allowed.
    ///
    /// If the seek operation completed successfully, this method returns the
    /// new position from the start of the stream. That position can be used
    /// later with `SeekFrom::Start`.
    ///
    /// # Errors
    /// Seeking to a negative offset is considered an error.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            SeekFrom::Start(i) => {
                self.set_position(i as usize);
                Ok(i)
            }
            SeekFrom::End(_) => unreachable!(),
            SeekFrom::Current(i) => {
                let new_pos = self.position() + i as usize;
                self.set_position(new_pos);
                Ok(new_pos as u64)
            }
        }
    }
}

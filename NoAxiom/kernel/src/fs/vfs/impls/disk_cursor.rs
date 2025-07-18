use alloc::{boxed::Box, sync::Arc, vec::Vec};

use async_trait::async_trait;
use driver::devices::block::BlockDevice;
use fatfs::*;

use crate::config::fs::BLOCK_SIZE;

#[derive(Clone)]
pub struct DiskCursor {
    blk: Arc<&'static dyn BlockDevice>,
    blk_id: usize,
    offset: usize,
}

impl DiskCursor {
    pub fn new(blk: Arc<&'static dyn BlockDevice>, blk_id: usize, offset: usize) -> Self {
        Self {
            blk,
            blk_id,
            offset,
        }
    }
    pub fn move_cursor(&mut self, offset: usize) {
        self.set_position(self.position() + offset)
    }
    pub fn position(&self) -> usize {
        self.blk_id * BLOCK_SIZE + self.offset
    }
    pub fn set_position(&mut self, pos: usize) {
        self.blk_id = pos / BLOCK_SIZE;
        self.offset = pos % BLOCK_SIZE;
    }

    /// mention that ext4_rs use 4k block size
    /// so we use 4 times read 512 block to represent 4k block
    ///
    /// only use for ext4_rs
    ///
    ///
    /// block:
    /// |------------------------------------|------------------------------------|------------------------------------|------------------------------------|
    ///
    /// request:
    ///         |---------------------------------------------------------------------------------------------------------------|
    /// offset  |   BLOCK_SIZE - offset      |
    pub async fn base_read_exact_block_size(&self, offset: usize) -> Vec<u8> {
        log::trace!("base_read_exact_block_size offset: {}", offset);
        let (blk, blk_id, offset) = (self.blk.clone(), offset / BLOCK_SIZE, offset % BLOCK_SIZE);

        const EXT4_RS_BLOCK_SIZE: usize = 4096;
        const BLK_NUMS: usize = EXT4_RS_BLOCK_SIZE / BLOCK_SIZE;

        let mut res = vec![0u8; EXT4_RS_BLOCK_SIZE];

        match offset {
            0 => {
                let mut data = [[0u8; BLOCK_SIZE]; BLK_NUMS];
                for i in 0..BLK_NUMS {
                    blk.read(blk_id + i, &mut data[i]).await.unwrap();
                }
                for i in 0..BLK_NUMS {
                    res[i * BLOCK_SIZE..(i + 1) * BLOCK_SIZE].copy_from_slice(&data[i]);
                }
            }
            _ => {
                let mut data = [[0u8; BLOCK_SIZE]; BLK_NUMS + 1];
                for i in 0..BLK_NUMS + 1 {
                    blk.read(blk_id + i, &mut data[i]).await.unwrap();
                }

                // sector 0
                res[..BLOCK_SIZE - offset].copy_from_slice(&data[0][offset..]);

                let offset = BLOCK_SIZE - offset;
                // sector 1 ~ BLK_NUMS - 1
                for i in 0..BLK_NUMS - 1 {
                    res[offset + i * BLOCK_SIZE..offset + (i + 1) * BLOCK_SIZE]
                        .copy_from_slice(&data[i + 1]);
                }

                // sector BLK_NUMS
                res[offset + (BLK_NUMS - 1) * BLOCK_SIZE..]
                    .copy_from_slice(&data[BLK_NUMS][..BLOCK_SIZE - offset]);
            }
        }
        log::trace!("base_read_exact_block_size ok");
        res
    }

    /// only use for ext4_rs
    ///
    ///  block:
    /// |------------------------------------|------------------------------------|------------------------------------|------------------------------------|
    ///
    /// request:
    ///         |------------------------------------------------------------|
    /// offset  |   BLOCK_SIZE - offset      |
    pub async fn base_write_exact(&self, offset: usize, buf: &[u8]) {
        log::trace!(
            "base_write_exact offset: {}, buf.len(): {}",
            offset,
            buf.len()
        );
        let blk = self.blk.clone();
        let (st_blk_id, st_offset) = (offset / BLOCK_SIZE, offset % BLOCK_SIZE);
        let ed_offset = offset + buf.len();
        let (ed_blk_id, ed_offset) = (ed_offset / BLOCK_SIZE, ed_offset % BLOCK_SIZE);

        log::trace!(
            "st_blk_id: {}, st_offset: {}, ed_blk_id: {}, ed_offset: {}, buf.len: {}",
            st_blk_id,
            st_offset,
            ed_blk_id,
            ed_offset,
            buf.len()
        );

        if st_blk_id == ed_blk_id {
            let mut data = [0u8; BLOCK_SIZE];
            blk.read(st_blk_id, &mut data).await.unwrap();
            data[st_offset..ed_offset].copy_from_slice(&buf);
            blk.write(st_blk_id, &data).await.unwrap();
            log::trace!("base_write_exact_offset ok");
            return;
        }

        // sector 0
        let mut data = [0u8; BLOCK_SIZE];
        blk.read(st_blk_id, &mut data).await.unwrap();
        data[st_offset..].copy_from_slice(&buf[..BLOCK_SIZE - st_offset]);
        blk.write(st_blk_id, &data).await.unwrap();

        // sector mid
        let mid_offset = BLOCK_SIZE - st_offset;
        for i in st_blk_id + 1..ed_blk_id {
            let mut data = [0u8; BLOCK_SIZE];
            let kth = i - st_blk_id - 1;
            data.copy_from_slice(
                &buf[mid_offset + kth * BLOCK_SIZE..mid_offset + (kth + 1) * BLOCK_SIZE],
            );
            blk.write(i, &data).await.unwrap();
        }

        // sector ed
        if ed_offset != 0 {
            let mut data = [0u8; BLOCK_SIZE];
            blk.read(ed_blk_id, &mut data).await.unwrap();
            data[..ed_offset].copy_from_slice(&buf[buf.len() - ed_offset..]);
            blk.write(ed_blk_id, &data).await.unwrap();
        }
        log::trace!("base_write_exact_offset ok");
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
        let mut data = [0u8; BLOCK_SIZE];
        self.blk.read(self.blk_id, &mut data).await.unwrap();
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
        let mut data = [0u8; BLOCK_SIZE];
        self.blk.read(self.blk_id, &mut data).await.unwrap();
        let write_size = (BLOCK_SIZE - self.offset).min(buf.len());
        data[self.offset..self.offset + write_size].copy_from_slice(&buf[..write_size]);
        self.blk.write(self.blk_id as usize, &data).await.unwrap();
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

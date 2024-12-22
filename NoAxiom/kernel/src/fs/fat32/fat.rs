//! `FAT` table

use alloc::vec::Vec;

use super::{bpb::BIOSParameterBlockOffset, ABC};
use crate::config::fs::FAT32_BYTES_PER_CLUSTER_ID;

pub struct FAT {
    /// count of `FAT` table
    fat_count: u8,
    /// `FAT1` start sector
    fat1_start: u32,
    sectors_per_fat: u32,
    bytes_per_sector_id: u16,
}

impl FAT {
    /// create a new `FAT` table
    pub fn new(sector: &[u8]) -> Self {
        type BPB = BIOSParameterBlockOffset;
        let fat_count = BPB::num_fats(sector);
        let fat1_start = BPB::reserved_sector_count(sector) as u32 + BPB::hidden_sector(sector);
        let sectors_per_fat = BPB::fat_size_32(sector);
        let bytes_per_sector_id = BPB::bytes_per_sector(sector);
        Self {
            fat_count,
            fat1_start,
            sectors_per_fat,
            bytes_per_sector_id,
        }
    }
    /// from `cluster_id` to `sector_id`
    pub fn sector_id(&self, cluster_id: u32) -> u32 {
        cluster_id * FAT32_BYTES_PER_CLUSTER_ID as u32 / self.bytes_per_sector_id as u32
            + self.fat1_start
    }
    /// from `cluster_id` to `sector_offset`
    pub fn sector_offset(&self, cluster_id: u32) -> u32 {
        (cluster_id * FAT32_BYTES_PER_CLUSTER_ID as u32) % self.bytes_per_sector_id as u32
    }
    /// find a free cluster, now is find the first free cluster  
    pub async fn find_free_cluster_id(&self, blk: ABC) -> Option<u32> {
        for sector_id in 0..self.sectors_per_fat {
            // read a sector
            let sector = blk
                .read_sector((self.fat1_start + sector_id) as usize)
                .await;
            // find a free cluster
            for (id, byte) in sector.read().data.chunks(4).enumerate() {
                if byte.iter().all(|x| *x == 0) {
                    return Some(
                        sector_id * self.bytes_per_sector_id as u32
                            / FAT32_BYTES_PER_CLUSTER_ID as u32
                            + id as u32,
                    );
                }
            }
        }
        None
    }
    /// set `cluster_id` to `val`
    pub async fn set_cluster_id(&self, blk: ABC, cluster_id: u32, val: u32) {
        let sector_id = self.sector_id(cluster_id);
        let sector_offset = self.sector_offset(cluster_id);
        let sector = blk.read_sector(sector_id as usize).await;
        // write `val` to `cluster_id`
        sector.write().data[sector_offset as usize..(sector_offset + 4) as usize]
            .copy_from_slice(&val.to_le_bytes());
        blk.write_sector(sector_id as usize, &sector.read().data)
            .await;
    }
    /// get `FAT` links by `first_cluster`
    pub async fn get_link(&self, blk: &ABC, first_cluster: u32) -> Vec<u32> {
        let mut res = Vec::new();
        let mut sector_id = self.sector_id(first_cluster) as usize;
        let mut sector_offset = self.sector_offset(first_cluster) as usize;
        res.push(first_cluster);
        loop {
            let sector = blk.read_sector(sector_id).await;
            let sector = sector.read().data;
            let value = u32::from_le_bytes(
                sector[sector_offset..(sector_offset + 4)]
                    .try_into()
                    .unwrap(),
            );
            match value {
                0x0 => {
                    panic!("FAT content can not equal to zero!");
                }
                0xffffff8..=0xfffffff => {
                    break;
                }
                id => {
                    sector_id = self.sector_id(id) as usize;
                    sector_offset = self.sector_offset(id) as usize;
                    res.push(id);
                }
            }
        }
        res
    }
}

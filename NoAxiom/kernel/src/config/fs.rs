//! File System configuration

/// Block size in bytes
pub const BLOCK_SIZE: usize = 512;

/// The size of a sector in bytes
pub const FAT32_SECTOR_SIZE: usize = BLOCK_SIZE;

/// The number of byte per cluster id
pub const FAT32_BYTES_PER_CLUSTER_ID: usize = 4;

/// The max LruCache size
pub const MAX_LRU_CACHE_SIZE: usize = 128;

pub const IS_DELETED: u8 = 0xe5;
pub const SPACE: u8 = 0x20;
pub const FIRST_CLUSTER: u32 = 2;

/// The number of wake ops when external interrupt comes
pub const WAKE_NUM: usize = 1;

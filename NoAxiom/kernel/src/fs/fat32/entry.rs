use alloc::{string::String, sync::Arc, vec::Vec};

use bit_field::BitField;

use super::{bpb::cluster_offset_sectors, fat::FAT, ABC};
use crate::config::fs::{BLOCK_SIZE, IS_DELETED, SPACE};

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Attribute {
    ATTR_READ_ONLY = 0x01,
    ATTR_HIDDEN = 0x02,
    ATTR_SYSTEM = 0x04,
    ATTR_VOLUME_ID = 0x08,
    ATTR_DIRECTORY = 0x10,
    ATTR_ARCHIVE = 0x20, // file
    ATTR_LONG_NAME = 0x0f,
}

impl Attribute {
    fn from(x: u8) -> Self {
        match x {
            0x01 => Self::ATTR_READ_ONLY,
            0x02 => Self::ATTR_HIDDEN,
            0x04 => Self::ATTR_SYSTEM,
            0x08 => Self::ATTR_VOLUME_ID,
            0x10 => Self::ATTR_DIRECTORY,
            0x20 => Self::ATTR_ARCHIVE,
            0x0f => Self::ATTR_LONG_NAME,
            _ => panic!("invalid attirbute value"),
        }
    }
}

impl Default for Attribute {
    fn default() -> Self {
        Attribute::ATTR_ARCHIVE
    }
}

#[derive(Clone, Default)]
pub struct EntryTime(u16);
impl EntryTime {
    pub fn seconds(&self) -> u16 {
        self.0.get_bits(0..5) * 2
    }
    pub fn minutes(&self) -> u16 {
        self.0.get_bits(5..11)
    }
    pub fn hours(&self) -> u16 {
        self.0.get_bits(11..15)
    }
}

#[derive(Clone, Default)]
pub struct EntryDate(u16);
impl EntryDate {
    pub fn day(&self) -> u16 {
        self.0.get_bits(0..5)
    }
    pub fn month(&self) -> u16 {
        self.0.get_bits(5..9)
    }
    pub fn year(&self) -> u16 {
        self.0.get_bits(9..15) + 1980
    }
}

#[derive(Clone, Default)]
/// short directory entry
pub struct ShortDirectoryEntry {
    pub name: [u8; 8],
    pub extension: [u8; 3],
    /// file attribute
    pub attribute: Attribute,
    /// Windows NT reserved
    pub _reserved: u8,
    /// millisecond stamp at file creation EntryTime
    pub tenth: u8,
    pub create_time: EntryTime,
    pub create_date: EntryDate,
    /// last access EntryDate
    pub last_access_date: EntryDate,
    pub first_cluster_high: u16,
    pub last_write_time: EntryTime,
    pub last_write_date: EntryDate,
    pub first_cluster_low: u16,
    pub file_size: u32,
}

impl ShortDirectoryEntry {
    /// create a new `ShortDirectoryEntry` from a slice
    pub fn from(src: [u8; 32]) -> Self {
        let mut name = [0; 8];
        name.copy_from_slice(&src[0..8]);
        let mut extension = [0; 3];
        extension.copy_from_slice(&src[8..11]);
        let attribute = Attribute::from(src[11]);
        let _reserved = src[12];
        let tenth = src[13];
        let create_time = u16::from_le_bytes(src[14..16].try_into().unwrap());
        let create_date = u16::from_le_bytes(src[16..18].try_into().unwrap());
        let last_access_date = u16::from_le_bytes(src[18..20].try_into().unwrap());
        let first_cluster_high = u16::from_le_bytes(src[20..22].try_into().unwrap());
        let last_write_time = u16::from_le_bytes(src[22..24].try_into().unwrap());
        let last_write_date = u16::from_le_bytes(src[24..26].try_into().unwrap());
        let first_cluster_low = u16::from_le_bytes(src[26..28].try_into().unwrap());
        let file_size = u32::from_le_bytes(src[28..32].try_into().unwrap());

        let create_time = EntryTime(create_time);
        let create_date = EntryDate(create_date);
        let last_access_date = EntryDate(last_access_date);
        let last_write_time = EntryTime(last_write_time);
        let last_write_date = EntryDate(last_write_date);
        Self {
            name,
            extension,
            attribute,
            _reserved,
            tenth,
            create_time,
            create_date,
            last_access_date,
            first_cluster_high,
            last_write_time,
            last_write_date,
            first_cluster_low,
            file_size,
        }
    }
    /// convert the `ShortDirectoryEntry` to a slice
    pub fn as_slice(&self) -> [u8; 32] {
        let mut res = [0; 32];
        res[0..8].copy_from_slice(&self.name);
        res[8..11].copy_from_slice(&self.extension);
        res[11] = self.attribute as u8;
        res[12] = self._reserved;
        res[13] = self.tenth;
        res[14..16].copy_from_slice(&self.create_time.0.to_le_bytes());
        res[16..18].copy_from_slice(&self.create_date.0.to_le_bytes());
        res[18..20].copy_from_slice(&self.last_access_date.0.to_le_bytes());
        res[20..22].copy_from_slice(&self.first_cluster_high.to_le_bytes());
        res[22..24].copy_from_slice(&self.last_write_time.0.to_le_bytes());
        res[24..26].copy_from_slice(&self.last_write_date.0.to_le_bytes());
        res[26..28].copy_from_slice(&self.first_cluster_low.to_le_bytes());
        res[28..32].copy_from_slice(&self.file_size.to_le_bytes());
        res
    }
    /// get the first cluster of the file
    pub fn first_cluster(&self) -> u32 {
        (self.first_cluster_high as u32) << 16 | self.first_cluster_low as u32
    }
    /// get the **full** name of the file
    pub fn name(&self) -> String {
        let name = String::from_utf8(self.name.to_vec()).unwrap();
        let mut has_extension = false;
        let mut extension = String::new();
        for c in self.extension.iter() {
            extension.push(*c as char);
            if *c != SPACE {
                has_extension = true;
            }
        }
        match has_extension {
            true => format!("{}.{}", name, extension),
            false => name,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.name[0] == IS_DELETED
    }
    pub fn is_free(&self) -> bool {
        self.name[0] == 0
    }

    /// `.`
    pub fn is_dot(&self) -> bool {
        self.name[0] == '.' as u8 && self.name[1..].iter().all(|c| *c == SPACE)
    }

    /// `..`
    pub fn is_dotdot(&self) -> bool {
        self.name[0] == '.' as u8
            && self.name[1] == '.' as u8
            && self.name[2..].iter().all(|c| *c == SPACE)
    }

    /// get the checksum of the short name
    pub fn checksum(&self) -> u8 {
        let mut sum = 0;
        for c in self.name.iter() {
            let temp = (sum >> 1) + *c;
            if sum.get_bit(0) {
                sum += 0x80;
            }
            sum += temp;
        }
        for c in self.extension.iter() {
            let temp = (sum >> 1) + *c;
            if sum.get_bit(0) {
                sum += 0x80;
            }
            sum += temp;
        }
        sum
    }

    /// get clusters owned by this entry
    pub async fn clusters(&self, blk: &Arc<ABC>, fat: &Arc<FAT>) -> Vec<u32> {
        fat.get_link(blk, self.first_cluster()).await
    }

    /// 读取该目录项占据的块设备数据
    pub async fn load(
        &self,
        blk: &Arc<ABC>,
        fat: &Arc<FAT>,
        bpb: &Arc<[u8; BLOCK_SIZE]>,
    ) -> Vec<u8> {
        let first_cluster = self.first_cluster();
        let clusters_link = fat.get_link(blk, first_cluster).await;
        let mut res = Vec::new();
        for cluster in clusters_link {
            let cluster = cluster_offset_sectors(&**bpb, cluster);
            let sector = blk.read_sector(cluster as usize).await;
            let sector = sector.read().data;
            sector.iter().for_each(|b| res.push(*b));
        }
        res
    }
}

// todo: LongDirectoryEntry

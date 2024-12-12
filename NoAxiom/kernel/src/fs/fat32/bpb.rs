//! The First Sector of FAT32

/// BootSector 偏移
pub enum BootSectorOffset {
    /// Jump instruction to boot code
    JmpBoot = 0,
    /// OEM name
    OEMName = 3,
    /// 驱动器编号
    /// Drive number
    DrvNum = 64,
    /// Reserved
    Reserved1 = 65,
    /// Extended boot signature
    BootSig = 66,
    /// Volume serial number
    VolID = 67,
    /// Volume label
    VolLab = 71,
    /// One of the strings "FAT12", "FAT16", "FAT32"
    FilSysType = 82,
}

/// BPB 偏移
pub enum BIOSParameterBlockOffset {
    /// Count of bytes per sector
    /// This value may take on only the following values: 512, 1024, 2048 or
    /// 4096
    BytsPerSec = 11,
    /// Number of sectors per allocation unit
    /// The legal values are 1, 2, 4, 8
    SecPerClus = 13,
    /// Number of reserved sectors in the Reserved region of the volume
    RsvdSecCnt = 14,
    /// The count of FAT data structures on the volume
    NumFATs = 16,
    /// For FAT32 volumes, this field must be set to 0
    RootEntCnt = 17,
    /// Old 16-bit total count of sectors on the volume
    TotSec16 = 19,
    /// ignored
    Media = 21,
    /// On FAT32 volumes this field mut be 0
    FATSz16 = 22,
    /// ignored
    SecPerTrk = 24,
    /// ignored
    NumHeads = 26,
    /// Count of hidden sectors preceding the partition that contains this FAT
    /// volume
    HiddSec = 28,
    /// The new 32-bit total count of sectors on the volume
    TotSec32 = 32,
    /// FAT32 32-bit count of sectors occupied by ONE FAT
    FATSz32 = 36,
    /// Extern Flags
    ExtFlags = 40,
    /// High bype is major revision number.
    /// Low byte is minor revision number.
    FSVer = 42,
    /// The cluster number of the first cluster of the root directory
    /// Usually 2 but not required to be 2.
    RootClus = 44,
    /// Sector number of FSINFO structure in the reserved area of the FAT32
    /// volume Usually 1
    FSInfo = 48,
    /// ignored
    BkBootSec = 50,
    /// Reserved
    Reserved = 52,
}

macro_rules! get {
    ($part:ty, $func_name:ident, $name:ident, $type:ty) => {
        pub fn $func_name(sector: &[u8]) -> $type {
            let offset = <$part>::$name as usize;
            <$type>::from_le_bytes(
                sector[offset..offset + core::mem::size_of::<$type>()]
                    .try_into()
                    .unwrap(),
            )
        }
    };
}

impl BootSectorOffset {
    get!(Self, jump_boot, JmpBoot, u32);
    get!(Self, oem_name, OEMName, u64);
    get!(Self, drv_num, DrvNum, u8);
    get!(Self, reserved1, Reserved1, u8);
    get!(Self, boot_sig, BootSig, u8);
    get!(Self, vol_id, VolID, u32);
}

impl BIOSParameterBlockOffset {
    get!(Self, bytes_per_sector, BytsPerSec, u16);
    get!(Self, sector_per_cluster, SecPerClus, u8);
    get!(Self, reserved_sector_count, RsvdSecCnt, u16);
    get!(Self, num_fats, NumFATs, u8);
    get!(Self, root_entry_cnt, RootEntCnt, u16); // assert_eq!(root_entry_cnt, 0);
    get!(Self, tot_sector_16, TotSec16, u16);
    get!(Self, media, Media, u8);
    get!(Self, fat_size_16, FATSz16, u16); // assert_eq!(fat_size_16, 0);
    get!(Self, sector_per_trk, SecPerTrk, u16);
    get!(Self, num_heads, NumHeads, u16);
    get!(Self, hidden_sector, HiddSec, u32);
    get!(Self, tot_sector_32, TotSec32, u32);
    get!(Self, fat_size_32, FATSz32, u32);
    get!(Self, extern_flags, ExtFlags, u16);
    get!(Self, files_ystem_version, FSVer, u16);
    get!(Self, root_cluster, RootClus, u32);
    get!(Self, fs_info, FSInfo, u16);
    get!(Self, bk_boot_sec, BkBootSec, u16);
    get!(Self, reserved, Reserved, u8);
}

/// get sector offset by cluster id
pub(crate) fn cluster_offset_sectors(sector: &[u8], cluster: u32) -> u32 {
    type BPB = BIOSParameterBlockOffset;
    BPB::reserved_sector_count(sector) as u32
        + BPB::hidden_sector(sector)
        + BPB::num_fats(sector) as u32 * BPB::fat_size_32(sector)
        + (cluster - 2) * BPB::sector_per_cluster(sector) as u32 // cluster 0 &
                                                                 // 1 is reserved
}

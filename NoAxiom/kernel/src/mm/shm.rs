use alloc::{collections::BTreeMap, vec::Vec};

use config::mm::SHM_OFFSET;
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;
use memory::address::{PhysAddr, VirtAddr};

use super::map_area::MapArea;
use crate::{
    cpu::current_task,
    include::{fs::CreateMode, mm::ShmIdDs},
    time::gettime::get_time,
};

lazy_static! {
    pub static ref SHM_MANAGER: SpinLock<ShmManager> = SpinLock::new(ShmManager::new());
}
pub struct ShmManager {
    shm_areas: BTreeMap<usize, ShmArea>,
}
pub struct ShmArea {
    shmid_ds: ShmIdDs,
    buffer: Vec<u8>,
}
pub struct ShmTracker {
    pub key: usize,
}
impl ShmTracker {
    pub fn new(key: usize) -> Self {
        SHM_MANAGER.lock().base_attach(key);
        Self { key }
    }
}
impl Drop for ShmTracker {
    fn drop(&mut self) {
        SHM_MANAGER.lock().base_detach(self.key);
    }
}
impl ShmManager {
    pub fn new() -> Self {
        Self {
            shm_areas: BTreeMap::new(),
        }
    }
    pub fn create(&mut self, key: usize, size: usize, shmflags: usize) -> usize {
        let key = if key == 0 {
            if self.shm_areas.is_empty() {
                1
            } else {
                self.shm_areas.last_key_value().unwrap().0 + 1
            }
        } else {
            key
        };
        let pid = current_task().unwrap().tid();
        let perm = CreateMode::from_bits((shmflags & 0o777) as u32).unwrap();
        let shmid_ds = ShmIdDs {
            shm_perm: perm,
            shm_size: size,
            shm_atime: 0,
            shm_dtime: 0,
            shm_ctime: get_time(),
            shm_cpid: pid,
            shm_lpid: 0,
            shm_nattch: 0,
        };
        let buffer: Vec<u8> = vec![0 as u8; size];
        let shm_area = ShmArea { shmid_ds, buffer };
        assert!(self.shm_areas.get(&key).is_none());
        self.shm_areas.insert(key, shm_area);
        key
    }
    pub fn base_attach(&mut self, key: usize) {
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        shm_area.shmid_ds.shm_nattch += 1;
    }
    pub fn base_detach(&mut self, key: usize) {
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        shm_area.shmid_ds.shm_nattch -= 1;
    }
    // pub fn attach(&mut self, key: usize, pid: usize) {
    //     let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
    //     // shm_area.shmid_ds.shm_atime = get_time();
    //     // shm_area.shmid_ds.shm_lpid = pid;
    //     shm_area.shmid_ds.shm_nattch += 1;
    // }
    // pub fn detach(&mut self, key: usize) {
    //     let pid = current_task().unwrap().tid();
    //     let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
    //     // shm_area.shmid_ds.shm_dtime = get_time();
    //     // shm_area.shmid_ds.shm_lpid = pid;
    //     shm_area.shmid_ds.shm_nattch -= 1;
    // }
    pub fn remove(&mut self, key: usize) {
        let shm_area = &mut self.shm_areas.get_mut(&key).unwrap();
        if shm_area.shmid_ds.shm_nattch == 0 {
            info!("shm {key} get removed");
            self.shm_areas.remove(&key);
        };
    }
    pub fn get_address_and_size(&self, key: usize) -> (PhysAddr, usize) {
        let shm_area = &self.shm_areas.get(&key).unwrap();
        let size = shm_area.shmid_ds.shm_size;
        (
            VirtAddr::from(shm_area.buffer.as_ptr() as usize).kernel_translate_into_pa(),
            size,
        )
    }
    pub fn get_nattch(&self, key: usize) -> usize {
        let shm_area = &self.shm_areas.get(&key).unwrap();
        shm_area.shmid_ds.shm_nattch
    }
}

pub struct ShmInfo {
    pub shm_areas: Vec<MapArea>,
    pub shm_trackers: BTreeMap<VirtAddr, ShmTracker>,
    pub shm_top: usize,
}

impl ShmInfo {
    pub fn new() -> Self {
        Self {
            shm_areas: Vec::new(),
            shm_trackers: BTreeMap::new(),
            shm_top: SHM_OFFSET,
        }
    }
}

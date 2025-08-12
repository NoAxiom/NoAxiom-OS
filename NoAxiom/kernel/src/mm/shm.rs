use alloc::{collections::BTreeMap, vec::Vec};

use config::mm::SHM_OFFSET;
use include::errno::{Errno, SysResult};
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;
use memory::address::{PhysAddr, VirtAddr};

use super::map_area::MapArea;
use crate::{
    cpu::current_task,
    include::{
        fs::CreateMode,
        ipc::{IpcGetFlags, IpcPerm, ShmIdDs, IPC_NEW},
    },
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
    fn new_area(key: usize, size: usize, shmflags: usize) -> ShmArea {
        assert!(key != IPC_NEW);
        let task = current_task().unwrap();
        let pid = task.pid();
        let mode = CreateMode::from_bits((shmflags & 0o777) as u32).unwrap();
        let user_id = task.user_id();
        let uid = user_id.uid();
        let gid = user_id.gid();
        let perm = IpcPerm {
            __key: key,
            uid,
            gid,
            cuid: uid,
            cgid: gid,
            mode,
            __seq: 0,
            __pad2: 0,
            __glibc_reserved1: 0,
            __glibc_reserved2: 0,
        };
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
        ShmArea { shmid_ds, buffer }
    }
    pub fn get(&mut self, mut key: usize, size: usize, shmflags: usize) -> SysResult<usize> {
        let flags = if key == IPC_NEW {
            key = self.shm_areas.last_key_value().map(|x| x.0).unwrap_or(&0) + 1;
            IpcGetFlags::IPC_CREAT | IpcGetFlags::IPC_EXCL
        } else {
            IpcGetFlags::from_bits_truncate(shmflags)
        };
        if self.shm_areas.contains_key(&key) {
            if flags.contains(IpcGetFlags::IPC_CREAT) {
                if flags.contains(IpcGetFlags::IPC_EXCL) {
                    Err(Errno::EEXIST)
                } else {
                    Ok(key)
                }
            } else {
                Err(Errno::ENOENT)
            }
        } else {
            let area = Self::new_area(key, size, shmflags);
            self.shm_areas.insert(key, area);
            Ok(key)
        }
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

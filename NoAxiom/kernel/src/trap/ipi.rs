//! inter-process interrupt handler

use alloc::sync::Arc;
use core::mem::swap;

use arch::{Arch, ArchMemory, ArchSbi};
use array_init::array_init;
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use crate::{config::cpu::CPU_NUM, cpu::get_hartid};

#[derive(Clone)]
pub enum IpiType {
    None,
    LoadBalance,
    TlbShootdown,
}

#[repr(align(64))]
pub struct IpiInfo {
    pub ipi_type: IpiType,
    pub from_hartid: usize,
}

impl IpiInfo {
    pub fn new(ipi_type: IpiType, from_hartid: usize) -> Self {
        Self {
            ipi_type,
            from_hartid,
        }
    }
    pub fn fetch(&mut self) -> Self {
        let mut ipi_type = IpiType::None;
        swap(&mut self.ipi_type, &mut ipi_type);
        Self {
            ipi_type,
            from_hartid: self.from_hartid,
        }
    }
}
impl Default for IpiInfo {
    fn default() -> Self {
        Self {
            ipi_type: IpiType::None,
            from_hartid: 0,
        }
    }
}

lazy_static! {
    pub static ref IPI_MANAGER: [Arc<SpinLock<IpiInfo>>; CPU_NUM] =
        array_init(|_| Arc::new(SpinLock::new(IpiInfo::default())));
}

pub fn current_ipi_info() -> IpiInfo {
    IPI_MANAGER[get_hartid()].lock().fetch()
}

pub fn send_ipi(to_hartid: usize, ipi_type: IpiType) {
    match ipi_type {
        IpiType::None => {
            return;
        }
        _ => {
            let from_hartid = get_hartid();
            *IPI_MANAGER[to_hartid].lock() = IpiInfo::new(ipi_type, from_hartid);
        }
    }
    Arch::send_ipi(to_hartid);
}

pub fn ipi_handler() {
    let info = current_ipi_info();
    trace!("ipi handler, from_hartid: {}", info.from_hartid);
    match info.ipi_type {
        IpiType::TlbShootdown => {
            info!("[IPI] tlb shootdown");
            Arch::tlb_flush();
        }
        IpiType::LoadBalance => {
            info!("[IPI] load balance done, current hart is woken!!");
        }
        _ => {
            info!("[IPI] unsupported ipi type");
        }
    }
    Arch::clear_ipi();
}

// pub fn send_ipi_test() {
//     let boot_hart_id =
// crate::entry::init::BOOT_HART_ID.load(core::sync::atomic::Ordering::SeqCst);
//     let from_hartid = get_hartid();
//     if from_hartid != boot_hart_id {
//         return;
//     }

//     debug!("send ipi test begin!");
//     for to_hartid in 0..CPU_NUM {
//         if to_hartid == from_hartid {
//             continue;
//         }
//         debug!("send ipi to hart {}", to_hartid);
//         send_ipi(to_hartid, IpiType::TlbShootdown);
//     }
//     debug!("send ipi test done!");
// }

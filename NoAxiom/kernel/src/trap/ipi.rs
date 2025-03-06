//! inter-process interrupt handler

use alloc::sync::Arc;
use core::task::Waker;

use arch::{Arch, ArchMemory, ArchSbi};
use array_init::array_init;
use ksync::mutex::SpinLock;
use lazy_static::lazy_static;

use crate::{config::arch::CPU_NUM, cpu::get_hartid, entry::init::BOOT_HART_ID};

#[derive(Clone)]
pub enum IpiType {
    None,
    Resched { waker: Waker },
    TlbShootdown,
}

impl IpiType {
    pub fn peek(&mut self) -> IpiType {
        self.clone()
    }
    pub fn fetch(&mut self) -> IpiType {
        let res = self.clone();
        *self = IpiType::None;
        res
    }
}

lazy_static! {
    pub static ref IPI_MANAGER: [Arc<SpinLock<IpiType>>; CPU_NUM] =
        array_init(|_| Arc::new(SpinLock::new(IpiType::None)));
}

pub fn current_ipi_type() -> IpiType {
    IPI_MANAGER[get_hartid()].lock().fetch()
}

pub fn send_ipi(hartid: usize, ipi_type: IpiType) {
    match ipi_type {
        IpiType::None => {
            return;
        }
        _ => {
            *IPI_MANAGER[get_hartid()].lock() = ipi_type;
        }
    }
    assert!(!matches!(
        IPI_MANAGER[get_hartid()].lock().peek(),
        IpiType::None
    ));
    Arch::send_ipi(hartid);
}

pub fn ipi_handler() {
    // warning: do not try to print in this function!!!
    match current_ipi_type() {
        IpiType::Resched { waker } => {
            waker.wake();
        }
        IpiType::TlbShootdown => {
            Arch::tlb_flush();
        }
        _ => {}
    }
    Arch::clear_ipi();
}

pub fn send_ipi_test() {
    let boot_hart_id = BOOT_HART_ID.load(core::sync::atomic::Ordering::SeqCst);
    let hartid = get_hartid();
    if hartid != boot_hart_id {
        return;
    }

    debug!("send ipi test begin!");
    for i in 0..CPU_NUM {
        if i == hartid {
            continue;
        }
        debug!("send ipi to hart {}", i);
        send_ipi(hartid, IpiType::TlbShootdown);
        assert!(matches!(
            IPI_MANAGER[get_hartid()].lock().peek(),
            IpiType::None
        ));
    }
    debug!("send ipi test done!");
}

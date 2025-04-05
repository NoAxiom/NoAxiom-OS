use alloc::boxed::Box;

use arch::consts::KERNEL_ADDR_OFFSET;
use async_trait::async_trait;
use config::mm::PAGE_SIZE;
use isomorphic_drivers::{
    block::ahci::{AHCI, BLOCK_SIZE},
    provider,
};
use log::info;
use pci::*;
use spin::Mutex;

use crate::{
    device::block::BlockDevice,
    mm::{
        address::PhysAddr,
        frame::{frame_alloc, frame_dealloc},
    },
};
pub struct SataBlock(Mutex<AHCI<Provider>>);

impl SataBlock {
    pub fn new() -> Self {
        Self(Mutex::new(pci_init().expect("AHCI new failed")))
    }
}

#[async_trait]
impl BlockDevice for SataBlock {
    async fn read(&self, block_id: usize, buf: &mut [u8]) {
        self.0.lock().read_block(block_id, buf);
    }
    async fn write(&self, block_id: usize, buf: &[u8]) {
        self.0.lock().write_block(block_id, buf);
    }
    async fn sync_all(&self) {}
}

pub struct Provider;

impl provider::Provider for Provider {
    const PAGE_SIZE: usize = PAGE_SIZE;
    fn alloc_dma(size: usize) -> (usize, usize) {
        let pages = size / PAGE_SIZE;
        let mut base = 0;
        for i in 0..pages {
            let frame = frame_alloc();
            let frame_pa: PhysAddr = frame.ppn().into();
            let frame_pa = frame_pa.into();
            core::mem::forget(frame);
            if i == 0 {
                base = frame_pa;
            }
            assert_eq!(frame_pa, base + i * PAGE_SIZE);
        }
        let base_page = base / PAGE_SIZE;
        info!("virtio_dma_alloc: {:#x} {}", base_page, pages);
        (base, base)
    }

    fn dealloc_dma(va: usize, size: usize) {
        info!("dealloc_dma: {:x} {:x}", va, size);
        let pages = size / PAGE_SIZE;
        let mut pa = va;
        for _ in 0..pages {
            frame_dealloc(PhysAddr::from(pa).into());
            pa += PAGE_SIZE;
        }
    }
}

// 扫描pci设备
const PCI_CONFIG_ADDRESS: usize = 0x20000000 | KERNEL_ADDR_OFFSET;
const PCI_COMMAND: u16 = 0x04;

struct UnusedPort;
impl PortOps for UnusedPort {
    unsafe fn read8(&self, _port: u16) -> u8 {
        0
    }
    unsafe fn read16(&self, _port: u16) -> u16 {
        0
    }
    unsafe fn read32(&self, _port: u16) -> u32 {
        0
    }
    unsafe fn write8(&self, _port: u16, _val: u8) {}
    unsafe fn write16(&self, _port: u16, _val: u16) {}
    unsafe fn write32(&self, _port: u16, _val: u32) {}
}

unsafe fn enable(loc: Location) {
    let ops = &UnusedPort;
    let am = CSpaceAccessMethod::MemoryMapped;

    let orig = am.read16(ops, loc, PCI_COMMAND);
    // bit0     |bit1       |bit2          |bit3           |bit10
    // IO Space |MEM Space  |Bus Mastering |Special Cycles |PCI Interrupt Disable
    am.write32(ops, loc, PCI_COMMAND, (orig | 0x40f) as u32);
    // Use PCI legacy interrupt instead
    // IO Space | MEM Space | Bus Mastering | Special Cycles
    am.write32(ops, loc, PCI_COMMAND, (orig | 0xf) as u32);
}

unsafe fn assign_bar(loc: Location, bar_index: usize, base_addr: u32) {
    let ops = &UnusedPort;
    let am = CSpaceAccessMethod::MemoryMapped;

    // 写入 BAR 地址
    let bar_offset = 0x10 + (bar_index * 4) as u16; // BAR 的偏移量
    am.write32(ops, loc, bar_offset, base_addr);

    // 读取回写入的值，确认 BAR 地址是否生效
    let assigned_addr = am.read32(ops, loc, bar_offset);
    info!("Assigned BAR#{}: {:#x}", bar_index, assigned_addr);
}

pub fn pci_init() -> Option<AHCI<Provider>> {
    debug!("pci_init");
    for dev in unsafe {
        scan_bus(
            &UnusedPort,
            CSpaceAccessMethod::MemoryMapped,
            PCI_CONFIG_ADDRESS,
        )
    } {
        info!(
            "pci: {:02x}:{:02x}.{} {:#x} {:#x} ({} {}) irq: {}:{:?}",
            dev.loc.bus,
            dev.loc.device,
            dev.loc.function,
            dev.id.vendor_id,
            dev.id.device_id,
            dev.id.class,
            dev.id.subclass,
            dev.pic_interrupt_line,
            dev.interrupt_pin
        );
        dev.bars.iter().enumerate().for_each(|(index, bar)| {
            if let Some(BAR::Memory(pa, len, _, t)) = bar {
                info!("\tbar#{} (MMIO) {:#x} [{:#x}] [{:?}]", index, pa, len, t);
            } else if let Some(BAR::IO(pa, len)) = bar {
                info!("\tbar#{} (IO) {:#x} [{:#x}]", index, pa, len);
            }
        });
        if dev.id.class == 0x01 && dev.id.subclass == 0x0 {
            debug!("Found virtio-blk-pci device");
            unsafe {
                assign_bar(dev.loc, 1, 0x10000000); // 为 bar#1 分配地址
                assign_bar(dev.loc, 4, 0x10001000); // 为 bar#4 分配地址
            }
            // Mass storage class, SATA subclass
            if let Some(BAR::Memory(pa, len, ..)) = dev.bars[1] {
                if pa == 0 {
                    debug!("bar#1 is not assigned");
                    continue;
                }

                if let Some(x) = AHCI::new(pa as usize, len as usize) {
                    return Some(x);
                } else {
                    info!("create ahci failed");
                }
            }
        }
    }
    None
}

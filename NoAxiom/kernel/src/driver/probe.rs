use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::ops::Range;

use fdt::{standard_nodes::Compatible, Fdt};
use ksync::Once;

use crate::platform;

pub static PROBE: Once<Arc<Probe>> = Once::new();

pub struct ProbeInfo {
    pub name: String,
    pub base_addr: usize,
    pub irq: usize,
    pub compatible: String,
}

impl ProbeInfo {
    pub fn new(name: String, base_addr: usize, irq: usize, compatible: String) -> Self {
        Self {
            name,
            base_addr,
            irq,
            compatible,
        }
    }
}

pub struct Probe<'a> {
    dtb: Fdt<'a>,
}
impl Probe<'_> {
    pub fn init() {
        let dtb_ptr = platform::platform_dtb_ptr();
        let fdt = unsafe { Fdt::from_ptr(dtb_ptr as *const u8).unwrap() };
        let probe = Probe { dtb: fdt };
        PROBE.call_once(|| Arc::new(probe));
    }
    /// Get the base address and irq number of the uart device from the device
    /// tree.
    pub fn probe_uart(&self) -> Option<fdt::node::FdtNode> {
        match self.probe_common_for_node("uart") {
            Some(device_info) => Some(device_info),
            None => self.probe_common_for_node("serial"),
        }
    }
    pub fn probe_common_for_node(&self, device_name: &str) -> Option<fdt::node::FdtNode> {
        let node = self
            .dtb
            .all_nodes()
            .find(|node| node.name.starts_with(device_name))?;
        Some(node)
    }
    /// Get the base address and irq number of the rtc device from the device
    /// tree.
    pub fn probe_rtc(&self) -> Option<ProbeInfo> {
        self.probe_common("rtc")
    }
    /// Get the base address and irq number of the virtio devices from the
    /// device tree.
    pub fn probe_virtio(&self) -> Option<Vec<ProbeInfo>> {
        let mut virtio_devices = Vec::new();
        for node in self.dtb.all_nodes() {
            if node.name.starts_with("virtio_mmio") {
                let reg = node.reg()?.next()?;
                let paddr = reg.starting_address as usize;
                println!("name : {} , paddr : {:x} ", node.name, paddr);
                let irq = node.property("interrupts")?.value;
                let irq = u32::from_be_bytes(irq[0..4].try_into().ok()?);

                let compatible = node.compatible().map(Compatible::first).unwrap();

                virtio_devices.push(ProbeInfo::new(
                    String::from("virtio_mmio"),
                    paddr,
                    irq as usize,
                    compatible.to_string(),
                ));
            }
        }
        if virtio_devices.is_empty() {
            println!("There is no virtio-mmio device");
            None
        } else {
            debug!("Prob virtio success");
            Some(virtio_devices)
        }
    }
    pub fn probe_common(&self, device_name: &str) -> Option<ProbeInfo> {
        let node = self
            .dtb
            .all_nodes()
            .find(|node| node.name.starts_with(device_name))?;
        let reg = node.reg()?.next()?;
        let range = Range {
            start: reg.starting_address as usize,
            end: reg.starting_address as usize + reg.size.unwrap(),
        };
        println!("range start {:x} end {:x}", range.start, range.end);
        let irq = node.property("interrupts").unwrap().value;
        let irq = u32::from_be_bytes(irq[0..4].try_into().unwrap());
        let compatible = node.compatible().map(Compatible::first).unwrap();
        Some(ProbeInfo::new(
            device_name.to_string(),
            range.start,
            irq as usize,
            compatible.to_string(),
        ))
    }

    pub fn probe_sdio(&self) -> Option<ProbeInfo> {
        self.probe_common("sdio1")
    }
}

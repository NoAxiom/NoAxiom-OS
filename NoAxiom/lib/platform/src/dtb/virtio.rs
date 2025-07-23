use alloc::vec::Vec;

pub struct DtbVirtioRegion {
    pub start_addr: usize,
    pub size: usize,
}

impl DtbVirtioRegion {
    pub fn new(start_addr: usize, size: usize) -> Self {
        Self { start_addr, size }
    }
    pub fn end_addr(&self) -> usize {
        self.start_addr + self.size
    }
    pub fn simplified(&self) -> (usize, usize) {
        (self.start_addr, self.size)
    }
}

pub struct DtbVirtioInfo {
    pub mmio_regions: Vec<DtbVirtioRegion>,
    pub pci_ecam: Vec<usize>,
}

impl DtbVirtioInfo {
    pub fn new() -> Self {
        Self {
            mmio_regions: Vec::new(),
            pci_ecam: Vec::new(),
        }
    }
    pub fn normalize(&mut self) {
        self.mmio_regions
            .sort_by(|a, b| a.start_addr.cmp(&b.start_addr));
        self.pci_ecam.sort();
    }
    pub fn pci_ecam_base(&self) -> usize {
        self.pci_ecam[0]
    }
}

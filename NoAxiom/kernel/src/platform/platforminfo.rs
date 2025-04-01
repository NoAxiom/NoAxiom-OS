use alloc::{string::String, vec::Vec};
use core::{cmp::min, fmt::Debug, ops::Range};

use arch::{Arch, Platform};
use fdt::{node::FdtNode, Fdt};

use crate::include::info;

/// Platform basic information
#[derive(Clone)]
pub struct PlatformInfo {
    /// Platform model
    pub model: [u8; 32],
    /// Number of CPUs
    pub smp: usize,
    /// Memory range
    pub memory: Vec<Range<usize>>,
    /// PLIC information
    pub plic: Range<usize>,
    /// CLINT information
    pub clint: Range<usize>,
    /// Initrd information
    pub initrd: Option<Range<usize>>,
    /// Kernel command line
    pub bootargs: Option<[u8; 255]>,
    pub bootargs_len: usize,
}

impl Debug for PlatformInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let index = self.model.iter().position(|&x| x == 0).unwrap_or(32);
        let model = core::str::from_utf8(&self.model[..index]).unwrap();
        write!(
            f,
            "This is a device tree representation of a {} platform\n",
            model
        )
        .unwrap();
        write!(f, "SMP:    {}\n", self.smp).unwrap();
        for memory in &self.memory {
            write!(f, "Memory: {:#x}..{:#x}\n", memory.start, memory.end).unwrap();
        }
        write!(f, "PLIC:   {:#x}..{:#x}\n", self.plic.start, self.plic.end).unwrap();
        write!(
            f,
            "CLINT:  {:#x}..{:#x}\n",
            self.clint.start, self.clint.end
        )
        .unwrap();
        write!(f, "Initrd: {:#x?}\n", self.initrd).unwrap();
        let bootargs = self
            .bootargs
            .as_ref()
            .map(|x| core::str::from_utf8(&x[..self.bootargs_len]).unwrap());
        write!(f, "Bootargs: {:?}", bootargs).unwrap();
        Ok(())
    }
}

/// Get platform information from a device-tree
pub fn platform_info_from_dtb(ptr: usize) -> PlatformInfo {
    #[cfg(feature = "vf2")]
    let ptr = FDT.as_ptr() as usize;

    let fdt = unsafe { Fdt::from_ptr(ptr as *const u8).unwrap() };
    walk_dt(fdt)
}

// Walk the device-tree and get platform information
fn walk_dt(fdt: Fdt) -> PlatformInfo {
    let mut machine = PlatformInfo {
        model: [0; 32],
        smp: 0,
        memory: Vec::new(),
        plic: 0..0,
        clint: 0..0,
        initrd: None,
        bootargs: None,
        bootargs_len: 0,
    };
    // let x = fdt.root();
    // debug!("Device tree root node: {:?}", x);
    machine.smp = fdt.cpus().count();
    let res = fdt.chosen().bootargs().map(|x| {
        let mut tmp = [0; 255];
        let bootargs = x.as_bytes();
        let len = min(bootargs.len(), tmp.len());
        tmp[0..len].copy_from_slice(&bootargs[..len]);
        (tmp, len)
    });
    if let Some((bootargs, len)) = res {
        machine.bootargs = Some(bootargs);
        machine.bootargs_len = len;
    }

    #[cfg(target_arch = "loongarch64")]
    let model = "loongarch64-qemu".as_bytes();
    #[cfg(target_arch = "riscv64")]
    let model = fdt.root().model().as_bytes();

    let len = min(model.len(), machine.model.len());
    machine.model[0..len].copy_from_slice(&model[..len]);

    walk(fdt, &mut machine);

    machine
}

fn walk(fdt: Fdt, machine: &mut PlatformInfo) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            info!("   {}  {}", node.name, compatible.all().collect::<String>());
        } else {
            info!("   {}", node.name);
        }
        if node.name.starts_with(&Arch::memory_name()) {
            trace!("[fdt]: memory node {}", node.name);
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                machine.memory.push(Range {
                    start: x.starting_address as usize,
                    end: x.starting_address as usize + x.size.unwrap(),
                });
            })
        } else if node.name.starts_with(&Arch::plic_name()) {
            trace!("[fdt]: plic node {}", node.name);
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                machine.plic = Range {
                    start: x.starting_address as usize,
                    end: x.starting_address as usize + x.size.unwrap(),
                }
            })
        } else if node.name.starts_with(&Arch::clint_name()) {
            trace!("[fdt]: clint node {}", node.name);
            let reg = node.reg().unwrap();
            reg.for_each(|x| {
                machine.clint = Range {
                    start: x.starting_address as usize,
                    end: x.starting_address as usize + x.size.unwrap(),
                }
            })
        } else if node.name.starts_with(&Arch::chose_name()) {
            trace!("[fdt]: chose node {}", node.name);
            let initrd_start = node.property("linux,initrd-start");
            if initrd_start.is_none() {
                trace!("No initrd");
                continue;
            }
            let initrd_start = initrd_start.unwrap();
            let initrd_end = node.property("linux,initrd-end").unwrap();
            let initrd_start = initrd_start.as_usize().unwrap();
            let initrd_end = initrd_end.as_usize().unwrap();
            machine.initrd = Some(Range {
                start: initrd_start,
                end: initrd_end,
            });
        }
    }
}

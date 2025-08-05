mod arch;
pub(crate) mod basic;
mod dtb;
mod pci;
mod realize;

pub fn probe_device(dtb: usize) {
    dtb::dtb_init(dtb);
    realize::device_realize();
}

pub use arch::devconf;

mod arch;
pub(crate) mod basic;
mod dtb;
mod pci;
mod realize;

pub fn probe_device(dtb: usize) {
    log::info!("[device] init with dtb");
    dtb::dtb_init(dtb);
    log::info!("[device] probe done!");
}

pub fn realize_device() {
    log::info!("[device] realize devices");
    realize::device_realize();
    log::info!("[device] realize done!");
}

pub use arch::devconf;

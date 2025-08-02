pub(crate) mod manager;
pub(crate) mod realize;

pub fn device_init() {
    crate::device::realize::device_realize();
}

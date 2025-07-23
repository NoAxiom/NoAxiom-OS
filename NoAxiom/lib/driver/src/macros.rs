/// device_cast!(PCI_DEV, BlockDevice)
/// Arc::new(PCI_DEV.get().unwrap() as &'static dyn BlockDevice)
#[macro_export]
macro_rules! device_cast {
    ($name:ident, $parent:ident) => {
        Arc::new($name.get().unwrap() as &'static dyn $parent)
    };
}

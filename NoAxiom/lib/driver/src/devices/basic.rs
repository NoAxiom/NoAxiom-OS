pub trait Device {
    fn device_name(&self) -> &'static str;
}

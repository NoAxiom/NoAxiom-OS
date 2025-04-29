pub trait ArchInfo {
    const ARCH_NAME: &'static str = "unknown";
    fn arch_info_print() {}
}

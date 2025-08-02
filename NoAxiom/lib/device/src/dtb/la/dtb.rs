use crate::dtb::info::DtbInitializerType;

pub const FDT: &[u8] = include_bytes!("./2k1000.dtb");
pub fn get_dtb_initializer() -> DtbInitializerType {
    #[cfg(feature = "qemu")]
    {
        use arch::consts::KERNEL_ADDR_OFFSET;
        DtbInitializerType::Ptr(0x100000 | KERNEL_ADDR_OFFSET)
    }
    #[cfg(not(feature = "qemu"))]
    {
        // use alloc::vec::Vec;
        // let res = Vec::new();
        // DtbInitializerType::Config(res)
        DtbInitializerType::Ref(FDT)
    }
}

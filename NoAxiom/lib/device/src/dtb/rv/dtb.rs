use crate::dtb::info::DtbInitializerType;

pub fn get_dtb_initializer() -> DtbInitializerType {
    #[cfg(feature = "qemu")]
    {
        use crate::dtb::dtb_addr::get_dtb_addr;
        match get_dtb_addr() {
            Some(addr) => DtbInitializerType::Ptr(addr),
            None => panic!("DTB address not initialized"),
        }
    }

    #[cfg(not(feature = "qemu"))]
    {
        const FDT: &[u8] = include_bytes!("./jh7110-visionfive-v2.dtb");
        DtbInitializerType::Ref(FDT)
    }
}

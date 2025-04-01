use arch::{Arch, ArchFull};

#[repr(C)]
pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

impl Utsname {
    pub fn get() -> Self {
        Self {
            sysname: Utsname::str2u8("NoAxiom"),
            nodename: Utsname::str2u8("NoAxiom-host"),
            release: Utsname::str2u8("1.0.0"),
            version: Utsname::str2u8("NoAxiom-OS SMP 1.0.0"),
            machine: Utsname::str2u8(Arch::ARCH_NAME),
            domainname: Utsname::str2u8("none"),
        }
    }

    pub fn str2u8(str: &str) -> [u8; 65] {
        let mut arr: [u8; 65] = [0; 65];
        let cstr = str.as_bytes();
        let len = str.len();
        for i in 0..len {
            arr[i] = cstr[i];
        }
        arr
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

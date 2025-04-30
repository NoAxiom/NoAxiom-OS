use arch::{Arch, ArchInfo};
use strum::FromRepr;

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
            sysname: Utsname::str2u8("Linux"),
            nodename: Utsname::str2u8("NoAxiom"),
            release: Utsname::str2u8("5.0"),
            version: Utsname::str2u8("5.13"),
            machine: Utsname::str2u8(Arch::ARCH_NAME),
            domainname: Utsname::str2u8("Rust"),
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

#[repr(u32)]
#[derive(FromRepr, Eq, PartialEq, Debug, Copy, Clone)]
pub enum SyslogAction {
    CLOSE = 0,
    OPEN = 1,
    READ = 2,
    ReadAll = 3,
    ReadClear = 4,
    CLEAR = 5,
    ConsoleOff = 6,
    ConsoleOn = 7,
    ConsoleLevel = 8,
    SizeUnread = 9,
    SizeBuffer = 10,
    Unknown = 11,
}

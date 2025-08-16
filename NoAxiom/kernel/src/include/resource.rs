use crate::{include::result::Errno, syscall::SysResult};

#[derive(Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RlimitResource {
    CPU = 0,
    FSIZE = 1,
    DATA = 2,
    STACK = 3,
    CORE = 4,
    RSS = 5,
    NPROC = 6,
    NOFILE = 7,
    MEMLOCK = 8,
    AS = 9,
    LOCKS = 10,
    SIGPENDING = 11,
    MSGQUEUE = 12,
    NICE = 13,
    RTPRIO = 14,
    RTTIME = 15,
}

impl RlimitResource {
    pub fn from_u32(value: u32) -> SysResult<Self> {
        match value {
            0 => Ok(RlimitResource::CPU),
            1 => Ok(RlimitResource::FSIZE),
            2 => Ok(RlimitResource::DATA),
            3 => Ok(RlimitResource::STACK),
            4 => Ok(RlimitResource::CORE),
            5 => Ok(RlimitResource::RSS),
            6 => Ok(RlimitResource::NPROC),
            7 => Ok(RlimitResource::NOFILE),
            8 => Ok(RlimitResource::MEMLOCK),
            9 => Ok(RlimitResource::AS),
            10 => Ok(RlimitResource::LOCKS),
            11 => Ok(RlimitResource::SIGPENDING),
            12 => Ok(RlimitResource::MSGQUEUE),
            13 => Ok(RlimitResource::NICE),
            14 => Ok(RlimitResource::RTPRIO),
            15 => Ok(RlimitResource::RTTIME),
            _ => Err(Errno::EINVAL),
        }
    }
}

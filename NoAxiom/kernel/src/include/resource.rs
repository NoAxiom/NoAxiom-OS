use crate::{include::result::Errno, syscall::SysResult};

#[derive(Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Resource {
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

impl Resource {
    pub fn from_u32(value: u32) -> SysResult<Self> {
        match value {
            0 => Ok(Resource::CPU),
            1 => Ok(Resource::FSIZE),
            2 => Ok(Resource::DATA),
            3 => Ok(Resource::STACK),
            4 => Ok(Resource::CORE),
            5 => Ok(Resource::RSS),
            6 => Ok(Resource::NPROC),
            7 => Ok(Resource::NOFILE),
            8 => Ok(Resource::MEMLOCK),
            9 => Ok(Resource::AS),
            10 => Ok(Resource::LOCKS),
            11 => Ok(Resource::SIGPENDING),
            12 => Ok(Resource::MSGQUEUE),
            13 => Ok(Resource::NICE),
            14 => Ok(Resource::RTPRIO),
            15 => Ok(Resource::RTTIME),
            _ => Err(Errno::EINVAL),
        }
    }
}

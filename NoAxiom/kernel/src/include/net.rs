use bitflags::bitflags;

use super::result::Errno;

bitflags! {
    /// @brief 用于指定socket的关闭类型
    /// 参考：https://code.dragonos.org.cn/xref/linux-6.1.9/include/net/sock.h?fi=SHUTDOWN_MASK#1573
    pub struct ShutdownType: u8 {
        //RCV_SHUTDOWN（值为1）：表示接收方向的关闭。当设置此标志时，表示socket不再接收数据。
        const RCV_SHUTDOWN = 1;
        //SEND_SHUTDOWN（值为2）：表示发送方向的关闭。当设置此标志时，表示socket不再发送数据。
        const SEND_SHUTDOWN = 2;
        //SHUTDOWN_MASK（值为3）：这是一个掩码，用于同时检查接收和发送方向的关闭。由于它是RCV_SHUTDOWN和SEND_SHUTDOWN的位或（bitwise OR）结果，它可以用来检查socket是否在任一方向上被关闭。
        const SHUTDOWN_MASK = 3;
    }
}

/// @brief posix套接字类型的枚举(这些值与linux内核中的值一致)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixSocketType {
    Stream = 1,
    Datagram = 2,
    Raw = 3,
    Rdm = 4,
    SeqPacket = 5,
    Dccp = 6,
    Packet = 10,
    SockCloexec = 1 << 19,
}

impl TryFrom<usize> for PosixSocketType {
    type Error = Errno;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Stream),
            2 => Ok(Self::Datagram),
            3 => Ok(Self::Raw),
            4 => Ok(Self::Rdm),
            5 => Ok(Self::SeqPacket),
            6 => Ok(Self::Dccp),
            10 => Ok(Self::Packet),
            524288 => Ok(Self::SockCloexec),
            _ => Err(Self::Error::EINVAL),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SocketType {
    Raw,
    Tcp,
    Udp,
    Unix,
}

bitflags::bitflags! {
    #[derive(Default, Debug, Copy, Clone)]
    pub struct SocketOptions: u32 {
        const BLOCK = 1 << 0;
        const BROADCAST = 1 << 1;
        const MULTICAST = 1 << 2;
        const REUSEADDR = 1 << 3;
        const REUSEPORT = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum AddressFamily {
    AF_UNIX = 1,
    /// ipv4
    AF_INET = 2,
    /// ipv6
    AF_INET6 = 10,
}

impl TryFrom<u16> for AddressFamily {
    type Error = Errno;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::AF_UNIX),
            2 => Ok(Self::AF_INET),
            10 => Ok(Self::AF_INET6),
            _ => Err(Self::Error::EINVAL),
        }
    }
}

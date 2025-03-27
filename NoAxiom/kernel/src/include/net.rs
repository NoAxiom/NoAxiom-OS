use bitflags::bitflags;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address, Ipv6Address};

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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct SockAddrIpv4 {
    /// 地址协议族
    pub sin_family: u16,
    /// Ipv4 的端口
    pub sin_port: u16,
    /// Ipv4 的地址
    pub sin_addr: u32,
    /// 零位，用于后续扩展
    pub sin_zero: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(C)]
pub struct SockAddrIpv6 {
    /// 地址协议族
    pub sin6_family: u16,
    /// Ipv6 的端口
    pub sin6_port: u16,
    /// Ipv6 的流信息
    pub sin6_flowinfo: u32,
    /// Ipv6 的地址
    pub sin6_addr: [u8; 16],
    /// IPv6 的范围ID
    pub sin6_scope_id: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrUnix {
    pub sun_family: u16,
    pub sun_path: [u8; 108],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrLinklayer {
    pub sll_family: u16,
    pub sll_protocol: u16,
    pub sll_ifindex: u32,
    pub sll_hatype: u16,
    pub sll_pkttype: u8,
    pub sll_halen: u8,
    pub sll_addr: [u8; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrNetlink {
    nl_family: u16,
    nl_pad: u16,
    nl_pid: u32,
    nl_groups: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockAddrPlaceholder {
    pub family: u16,
    pub data: [u8; 14],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SockAddr {
    pub family: u16, // 地址协议族,用于match分配
    pub addr_ipv4: SockAddrIpv4,
    pub addr_ipv6: SockAddrIpv6,
    pub addr_unix: SockAddrUnix,
    pub addr_linklayer: SockAddrLinklayer,
    pub addr_netlink: SockAddrNetlink,
    pub addr_ph: SockAddrPlaceholder,
}

impl SockAddr {
    pub fn get_endpoint(&self) -> IpEndpoint {
        unsafe {
            match AddressFamily::try_from(self.family).unwrap() {
                AddressFamily::AF_INET => {
                    let addr = self.addr_ipv4.sin_addr;
                    let port = self.addr_ipv4.sin_port.to_be();
                    IpEndpoint::new(
                        IpAddress::Ipv4(Ipv4Address::from_bytes(&addr.to_be_bytes())),
                        port,
                    )
                }
                AddressFamily::AF_INET6 => {
                    let addr = self.addr_ipv6.sin6_addr;
                    let port = self.addr_ipv6.sin6_port.to_be();
                    IpEndpoint::new(IpAddress::Ipv6(Ipv6Address::from_bytes(&addr)), port)
                }
                AddressFamily::AF_UNIX => unreachable!(),
            }
        }
    }
    pub fn from_endpoint(endpoint: IpEndpoint) -> Self {
        match endpoint.addr {
            IpAddress::Ipv4(v4) => Self {
                addr_ipv4: SockAddrIpv4 {
                    sin_family: AddressFamily::AF_INET as u16,
                    sin_port: endpoint.port.to_be(),
                    sin_addr: u32::from_be_bytes(v4.0),
                    sin_zero: [0; 8],
                },
            },
            IpAddress::Ipv6(v6) => Self {
                addr_ipv6: SockAddrIpv6 {
                    sin6_family: AddressFamily::AF_INET6 as u16,
                    sin6_port: endpoint.port.to_be(),
                    sin6_flowinfo: 0,
                    sin6_addr: v6.0,
                    sin6_scope_id: 0,
                },
            },
        }
    }
}

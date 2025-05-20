use bitflags::bitflags;
use smoltcp::wire::{IpAddress, IpEndpoint, Ipv4Address, Ipv6Address};
use strum::FromRepr;

use super::result::Errno;
use crate::syscall::SysResult;

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
    pub fn new(addr: usize, addr_len: usize) -> SysResult<Self> {
        // todo: check addr!
        let family = AddressFamily::try_from(unsafe { *(addr as *const u16) })?;
        match family {
            AddressFamily::AF_INET => {
                if addr_len < core::mem::size_of::<SockAddrIpv4>() {
                    error!("[Sockaddr::new] AF_INET addrlen error");
                    return Err(Errno::EINVAL);
                }
                Ok(SockAddr {
                    addr_ipv4: unsafe { *(addr as *const _) },
                })
            }
            AddressFamily::AF_INET6 => {
                if addr_len < core::mem::size_of::<SockAddrIpv6>() {
                    error!("[Sockaddr::new] AF_INET6 addrlen error");
                    return Err(Errno::EINVAL);
                }
                Ok(SockAddr {
                    addr_ipv6: unsafe { *(addr as *const _) },
                })
            }
            AddressFamily::AF_UNIX => {
                warn!("[Sockaddr::new] is AF_UNIX!");
                if addr_len < core::mem::size_of::<SockAddrUnix>() {
                    error!("[Sockaddr::new] AF_UNIX addrlen error");
                    return Err(Errno::EINVAL);
                }
                Ok(SockAddr {
                    addr_unix: unsafe { *(addr as *const _) },
                })
            }
        }
    }

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

#[allow(non_camel_case_types)]
#[repr(i32)]
#[derive(FromRepr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixSocketOption {
    SO_DEBUG = 1,
    SO_REUSEADDR = 2,
    SO_TYPE = 3,
    SO_ERROR = 4,
    SO_DONTROUTE = 5,
    SO_BROADCAST = 6,
    SO_SNDBUF = 7,
    SO_RCVBUF = 8,
    SO_SNDBUFFORCE = 32,
    SO_RCVBUFFORCE = 33,
    SO_KEEPALIVE = 9,
    SO_OOBINLINE = 10,
    SO_NO_CHECK = 11,
    SO_PRIORITY = 12,
    SO_LINGER = 13,
    SO_BSDCOMPAT = 14,
    SO_REUSEPORT = 15,
    SO_PASSCRED = 16,
    SO_PEERCRED = 17,
    SO_RCVLOWAT = 18,
    SO_SNDLOWAT = 19,
    SO_RCVTIMEO_OLD = 20,
    SO_SNDTIMEO_OLD = 21,

    SO_SECURITY_AUTHENTICATION = 22,
    SO_SECURITY_ENCRYPTION_TRANSPORT = 23,
    SO_SECURITY_ENCRYPTION_NETWORK = 24,

    SO_BINDTODEVICE = 25,

    /// 与SO_GET_FILTER相同
    SO_ATTACH_FILTER = 26,
    SO_DETACH_FILTER = 27,

    SO_PEERNAME = 28,

    SO_ACCEPTCONN = 30,

    SO_PEERSEC = 31,
    SO_PASSSEC = 34,

    SO_MARK = 36,

    SO_PROTOCOL = 38,
    SO_DOMAIN = 39,

    SO_RXQ_OVFL = 40,

    /// 与SCM_WIFI_STATUS相同
    SO_WIFI_STATUS = 41,
    SO_PEEK_OFF = 42,

    /* Instruct lower device to use last 4-bytes of skb data as FCS */
    SO_NOFCS = 43,

    SO_LOCK_FILTER = 44,
    SO_SELECT_ERR_QUEUE = 45,
    SO_BUSY_POLL = 46,
    SO_MAX_PACING_RATE = 47,
    SO_BPF_EXTENSIONS = 48,
    SO_INCOMING_CPU = 49,
    SO_ATTACH_BPF = 50,
    // SO_DETACH_BPF = SO_DETACH_FILTER,
    SO_ATTACH_REUSEPORT_CBPF = 51,
    SO_ATTACH_REUSEPORT_EBPF = 52,

    SO_CNX_ADVICE = 53,
    SCM_TIMESTAMPING_OPT_STATS = 54,
    SO_MEMINFO = 55,
    SO_INCOMING_NAPI_ID = 56,
    SO_COOKIE = 57,
    SCM_TIMESTAMPING_PKTINFO = 58,
    SO_PEERGROUPS = 59,
    SO_ZEROCOPY = 60,
    /// 与SCM_TXTIME相同
    SO_TXTIME = 61,

    SO_BINDTOIFINDEX = 62,

    SO_TIMESTAMP_OLD = 29,
    SO_TIMESTAMPNS_OLD = 35,
    SO_TIMESTAMPING_OLD = 37,
    SO_TIMESTAMP_NEW = 63,
    SO_TIMESTAMPNS_NEW = 64,
    SO_TIMESTAMPING_NEW = 65,

    SO_RCVTIMEO_NEW = 66,
    SO_SNDTIMEO_NEW = 67,

    SO_DETACH_REUSEPORT_BPF = 68,

    SO_PREFER_BUSY_POLL = 69,
    SO_BUSY_POLL_BUDGET = 70,

    SO_NETNS_COOKIE = 71,
    SO_BUF_LOCK = 72,
    SO_RESERVE_MEM = 73,
    SO_TXREHASH = 74,
    SO_RCVMARK = 75,
}

#[repr(u16)]
#[derive(FromRepr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixIpProtocol {
    /// Dummy protocol for TCP.
    IP = 0,
    /// Internet Control Message Protocol.
    ICMP = 1,
    /// Internet Group Management Protocol.
    IGMP = 2,
    /// IPIP tunnels (older KA9Q tunnels use 94).
    IPIP = 4,
    /// Transmission Control Protocol.
    TCP = 6,
    /// Exterior Gateway Protocol.
    EGP = 8,
    /// PUP protocol.
    PUP = 12,
    /// User Datagram Protocol.
    UDP = 17,
    /// XNS IDP protocol.
    IDP = 22,
    /// SO Transport Protocol Class 4.
    TP = 29,
    /// Datagram Congestion Control Protocol.
    DCCP = 33,
    /// IPv6-in-IPv4 tunnelling.
    IPv6 = 41,
    /// RSVP Protocol.
    RSVP = 46,
    /// Generic Routing Encapsulation. (Cisco GRE) (rfc 1701, 1702)
    GRE = 47,
    /// Encapsulation Security Payload protocol
    ESP = 50,
    /// Authentication Header protocol
    AH = 51,
    /// Multicast Transport Protocol.
    MTP = 92,
    /// IP option pseudo header for BEET
    BEETPH = 94,
    /// Encapsulation Header.
    ENCAP = 98,
    /// Protocol Independent Multicast.
    PIM = 103,
    /// Compression Header Protocol.
    COMP = 108,
    /// Stream Control Transport Protocol
    SCTP = 132,
    /// UDP-Lite protocol (RFC 3828)
    UDPLITE = 136,
    /// MPLS in IP (RFC 4023)
    MPLSINIP = 137,
    /// Ethernet-within-IPv6 Encapsulation
    ETHERNET = 143,
    /// Raw IP packets
    RAW = 255,
    /// Multipath TCP connection
    MPTCP = 262,
}

#[repr(i32)]
#[derive(FromRepr, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosixTcpSocketOptions {
    /// Turn off Nagle's algorithm.
    NoDelay = 1,
    /// Limit MSS.
    MaxSegment = 2,
    /// Never send partially complete segments.
    Cork = 3,
    /// Start keeplives after this period.
    KeepIdle = 4,
    /// Interval between keepalives.
    KeepIntvl = 5,
    /// Number of keepalives before death.
    KeepCnt = 6,
    /// Number of SYN retransmits.
    Syncnt = 7,
    /// Lifetime for orphaned FIN-WAIT-2 state.
    Linger2 = 8,
    /// Wake up listener only when data arrive.
    DeferAccept = 9,
    /// Bound advertised window
    WindowClamp = 10,
    /// Information about this connection.
    Info = 11,
    /// Block/reenable quick acks.
    QuickAck = 12,
    /// Congestion control algorithm.
    Congestion = 13,
    /// TCP MD5 Signature (RFC2385).
    Md5Sig = 14,
    /// Use linear timeouts for thin streams
    ThinLinearTimeouts = 16,
    /// Fast retrans. after 1 dupack.
    ThinDupack = 17,
    /// How long for loss retry before timeout.
    UserTimeout = 18,
    /// TCP sock is under repair right now.
    Repair = 19,
    RepairQueue = 20,
    QueueSeq = 21,
    RepairOptions = 22,
    /// Enable FastOpen on listeners
    FastOpen = 23,
    Timestamp = 24,
    /// Limit number of unsent bytes in write queue.
    NotSentLowat = 25,
    /// Get Congestion Control (optional) info.
    CCInfo = 26,
    /// Record SYN headers for new connections.
    SaveSyn = 27,
    /// Get SYN headers recorded for connection.
    SavedSyn = 28,
    /// Get/set window parameters.
    RepairWindow = 29,
    /// Attempt FastOpen with connect.
    FastOpenConnect = 30,
    /// Attach a ULP to a TCP connection.
    ULP = 31,
    /// TCP MD5 Signature with extensions.
    Md5SigExt = 32,
    /// Set the key for Fast Open(cookie).
    FastOpenKey = 33,
    /// Enable TFO without a TFO cookie.
    FastOpenNoCookie = 34,
    ZeroCopyReceive = 35,
    /// Notify bytes available to read as a cmsg on read.
    /// 与TCP_CM_INQ相同
    INQ = 36,
    /// delay outgoing packets by XX usec
    TxDelay = 37,
}

//! ref:
//! Dragon OS -- NetDevice trait
mod loopback;
use alloc::{string::String, sync::Arc};

use ksync::{mutex::SpinLock, Once};
use loopback::LoopBackDev;
use smoltcp::{
    iface::{self, Interface},
    phy::{DeviceCapabilities, Loopback},
    time::Instant,
    wire::{self, EthernetAddress},
};

use crate::syscall::SysResult;

// todo: use dyn NetDevice
pub static LOOP_BACK: Once<Arc<LoopBackDev>> = Once::new();

pub fn init_net_device(net_device: Arc<LoopBackDev>) {
    LOOP_BACK.call_once(|| net_device);
}

// #[derive(Debug, Copy, Clone)]
// pub enum Operstate {
//     /// 网络接口的状态未知
//     IF_OPER_UNKNOWN = 0,
//     /// 网络接口不存在
//     IF_OPER_NOTPRESENT = 1,
//     /// 网络接口已禁用或未连接
//     IF_OPER_DOWN = 2,
//     /// 网络接口的下层接口已关闭
//     IF_OPER_LOWERLAYERDOWN = 3,
//     /// 网络接口正在测试
//     IF_OPER_TESTING = 4,
//     /// 网络接口处于休眠状态
//     IF_OPER_DORMANT = 5,
//     /// 网络接口已启用
//     IF_OPER_UP = 6,
// }

// bitflags::bitflags! {
//     pub struct NetDeivceState: u16 {
//         /// 表示网络设备已经启动
//         const __LINK_STATE_START = 1 << 0;
//         /// 表示网络设备在系统中存在，即注册到sysfs中
//         const __LINK_STATE_PRESENT = 1 << 1;
//         /// 表示网络设备没有检测到载波信号
//         const __LINK_STATE_NOCARRIER = 1 << 2;
//         /// 表示设备的链路监视操作处于挂起状态
//         const __LINK_STATE_LINKWATCH_PENDING = 1 << 3;
//         /// 表示设备处于休眠状态
//         const __LINK_STATE_DORMANT = 1 << 4;
//     }
// }

pub trait NetDevice: Send + Sync {
    /// @brief 获取网卡的MAC地址
    fn mac(&self) -> EthernetAddress;

    fn iface_name(&self) -> String;

    /// @brief 获取网卡的id
    fn nic_id(&self) -> usize;

    fn poll(&self, sockets: &mut iface::SocketSet) -> SysResult<()>;

    // fn update_ip_addrs(&self, ip_addrs: &[wire::IpCidr]) -> SysResult<()>;

    // @brief 获取smoltcp的网卡接口类型
    fn inner_iface(&self) -> &SpinLock<Interface>;
    // fn as_any_ref(&'static self) -> &'static dyn core::any::Any;

    // fn addr_assign_type(&self) -> u8;

    // fn net_device_type(&self) -> u16;

    // fn net_state(&self) -> NetDeivceState;

    // fn set_net_state(&self, state: NetDeivceState);

    // fn operstate(&self) -> Operstate;

    // fn set_operstate(&self, state: Operstate);
}

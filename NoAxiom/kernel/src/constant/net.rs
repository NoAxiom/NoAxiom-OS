pub const PORT_MAX: usize = 65535;

pub const DEFAULT_METADATA_BUF_SIZE: usize = 1024;
pub const DEFAULT_RX_BUF_SIZE: usize = 512 * 1024;
pub const DEFAULT_TX_BUF_SIZE: usize = 512 * 1024;

pub struct SocketConstants {
    pub default_metadata_buf_size: usize,
    pub default_rx_buf_size: usize,
    pub default_tx_buf_size: usize,
}

impl SocketConstants {
    pub const fn tcp() -> Self {
        SocketConstants {
            default_metadata_buf_size: 1024,
            default_rx_buf_size: 512 * 1024,
            default_tx_buf_size: 512 * 1024,
        }
    }
    pub const fn udp() -> Self {
        SocketConstants {
            default_metadata_buf_size: 1024,
            default_rx_buf_size: 64 * 1024,
            default_tx_buf_size: 64 * 1024,
        }
    }
}

pub const TCP_CONSTANTS: SocketConstants = SocketConstants::tcp();
pub const UDP_CONSTANTS: SocketConstants = SocketConstants::udp();

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
            default_rx_buf_size: 512 * 1024,
            default_tx_buf_size: 512 * 1024,
        }
    }
}

pub const TCP_CONSTANTS: SocketConstants = SocketConstants::tcp();
pub const UDP_CONSTANTS: SocketConstants = SocketConstants::udp();

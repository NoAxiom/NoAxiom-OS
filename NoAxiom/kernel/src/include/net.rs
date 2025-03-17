use bitflags::bitflags;

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

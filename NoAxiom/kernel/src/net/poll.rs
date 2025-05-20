//! ref: Drangon OS
use smoltcp::socket::{tcp, udp};

use crate::include::{io::PollEvent, net::ShutdownType};

/// ### 为socket提供无锁的poll方法
///
/// 因为在网卡中断中，需要轮询socket的状态，如果使用socket文件或者其inode来poll
/// 在当前的设计，会必然死锁，所以引用这一个设计来解决，提供无🔓的poll
pub struct SocketPollMethod;

impl SocketPollMethod {
    pub fn tcp_poll(socket: &tcp::Socket, shutdown: &ShutdownType) -> PollEvent {
        let mut events = PollEvent::empty();
        // debug!("enter tcp_poll! is_posix_listen:{}", is_posix_listen);

        let state = socket.state();

        if shutdown.bits() == ShutdownType::SHUTDOWN_MASK.bits() || state == tcp::State::Closed {
            events.insert(PollEvent::POLLHUP);
        }

        if shutdown.contains(ShutdownType::RCV_SHUTDOWN) {
            events.insert(PollEvent::POLLIN | PollEvent::POLLRDNORM | PollEvent::POLLRDHUP);
        }

        // Connected or passive Fast Open socket?
        if state != tcp::State::SynSent && state != tcp::State::SynReceived {
            // socket有可读数据
            if socket.can_recv() {
                events.insert(PollEvent::POLLIN | PollEvent::POLLRDNORM);
            }

            if !(shutdown.contains(ShutdownType::SEND_SHUTDOWN)) {
                // 缓冲区可写（这里判断可写的逻辑好像跟linux不太一样）
                if socket.send_queue() < socket.send_capacity() {
                    events.insert(PollEvent::POLLOUT | PollEvent::POLLWRNORM);
                } else {
                    // TODO：触发缓冲区已满的信号SIGIO
                    todo!("A signal SIGIO that the buffer is full needs to be sent");
                }
            } else {
                // 如果我们的socket关闭了SEND_SHUTDOWN，epoll事件就是EPOLLOUT
                events.insert(PollEvent::POLLOUT | PollEvent::POLLWRNORM);
            }
        } else if state == tcp::State::SynSent {
            events.insert(PollEvent::POLLOUT | PollEvent::POLLWRNORM);
        }

        // socket发生错误
        // TODO: 这里的逻辑可能有问题，
        // 需要进一步验证是否is_active()==false就代表socket发生错误
        if !socket.is_active() {
            events.insert(PollEvent::POLLERR);
        }

        events
    }

    pub fn udp_poll(socket: &udp::Socket, shutdown: &ShutdownType) -> PollEvent {
        let mut event = PollEvent::empty();

        if shutdown.contains(ShutdownType::RCV_SHUTDOWN) {
            event.insert(PollEvent::POLLRDHUP | PollEvent::POLLIN | PollEvent::POLLRDNORM);
        }
        if shutdown.contains(ShutdownType::SHUTDOWN_MASK) {
            event.insert(PollEvent::POLLHUP);
        }

        if socket.can_recv() {
            event.insert(PollEvent::POLLIN | PollEvent::POLLRDNORM);
        }

        if socket.can_send() {
            event.insert(PollEvent::POLLOUT | PollEvent::POLLWRNORM | PollEvent::POLLWRBAND);
        } else {
            // TODO: 缓冲区空间不够，需要使用信号处理
            todo!()
        }

        return event;
    }
}

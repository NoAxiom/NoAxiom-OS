use bitflags::bitflags;

use crate::{constant::io::FD_SET_LEN, utils::align_offset};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PollFd {
    /// Fd
    pub fd: i32,
    /// Requested events
    pub events: PollEvent,
    /// Returned events
    pub revents: PollEvent,
}

bitflags! {
    /// 可以被轮询的事件类型。
    ///
    /// 这些位可以在 `events` 中设置（参见 `ppoll()`）以指示感兴趣的事件类型；
    ///
    /// 它们将出现在 `revents` 中以指示文件描述符的状态。
    #[derive(Debug, Copy, Clone)]
    pub struct PollEvent: u16 {
        /// 有数据可读。
        const POLLIN = 0x001;
        /// 表示有紧急的数据可读，比如TCP socket的带外数据。。
        const POLLPRI = 0x002;
        /// 现在写入不会阻塞。
        const POLLOUT = 0x004;
        /// 表示发生了错误条件。。仅隐式轮询。总是隐式轮询的事件类型。这些位不需要在 `events` 中设置，但它们会出现在 `revents` 中以指示文件描述符的状态。
        const POLLERR = 0x008;
        /// 表示连接被挂断。仅隐式轮询。
        const POLLHUP = 0x010;
        /// 无效的轮询请求,表示文件描述符不是一个打开的文件。仅隐式轮询。
        const POLLNVAL = 0x020;
        // 这些值在 XPG4.2 中定义。
        /// 可以读取普通数据。
        const POLLRDNORM = 0x040;
        /// 可以读取优先级数据。
        const POLLRDBAND = 0x080;
        /// 现在写入不会阻塞。
        const POLLWRNORM = 0x100;
        /// 可以写入优先级数据。
        const POLLWRBAND = 0x200;
        /// Linux 扩展。
        const POLLMSG = 0x400;
        /// Linux 扩展。
        const POLLREMOVE = 0x1000;
        /// Linux 扩展。
        const POLLRDHUP = 0x2000;
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FdSet {
    fd_list: [u64; FD_SET_LEN],
}

impl FdSet {
    pub fn clear(&mut self) {
        for i in 0..FD_SET_LEN {
            self.fd_list[i] = 0;
        }
    }

    /// Add the given file descriptor to the collection. Calculate the index and
    /// corresponding bit of the file descriptor in the array, and set the bit
    /// to 1
    pub fn set(&mut self, fd: usize) {
        let (idx, bit) = align_offset(fd, core::mem::size_of::<u64>());
        self.fd_list[idx] |= 1 << bit;
    }

    /// Check if the given file descriptor is in the collection. Calculate the
    /// index and corresponding bit of the file descriptor in the array, and
    /// check if the bit is 1
    pub fn is_set(&self, fd: usize) -> bool {
        let (idx, bit) = align_offset(fd, core::mem::size_of::<u64>());
        self.fd_list[idx] & (1 << bit) != 0
    }
}

use alloc::{sync::Arc, vec::Vec};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use include::errno::Errno;

use super::{Syscall, SyscallResult};
use crate::{
    fs::vfs::basic::file::File,
    include::io::{PollEvent, PollFd},
    mm::user_ptr::UserPtr,
    signal::sig_set::SigSet,
    time::time_spec::TimeSpec,
    utils::futures::{TimeLimitedFuture, TimeLimitedType},
};

struct PpollItem {
    id: usize,
    events: PollEvent,
    file: Arc<dyn File>,
}

impl PpollItem {
    fn new(id: usize, events: PollEvent, file: Arc<dyn File>) -> Self {
        Self { id, events, file }
    }
}

struct PpollFuture {
    fds: Vec<PpollItem>,
}

impl PpollFuture {
    fn new(fds: Vec<PpollItem>) -> Self {
        Self { fds }
    }
}

impl Future for PpollFuture {
    type Output = Vec<(usize, PollEvent)>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut result = Vec::new();
        for poll_item in &self.fds {
            let id = poll_item.id;
            let req = &poll_item.events;
            let res = poll_item.file.poll(req);
            if !res.is_empty() {
                result.push((id, res));
            }
        }
        if result.is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(result)
        }
    }
}

impl Syscall<'_> {
    pub async fn sys_ppoll(
        &self,
        fds_ptr: usize,
        nfds: usize,
        timeout_ptr: usize,
        sigmask_ptr: usize,
    ) -> SyscallResult {
        let timeout_ptr = UserPtr::<TimeSpec>::new(timeout_ptr);
        let timeout = if timeout_ptr.is_null() {
            None
        } else {
            Some(Duration::from(timeout_ptr.read()))
        };

        let sigmask_ptr = UserPtr::<SigSet>::new(sigmask_ptr);
        let sigmask = if sigmask_ptr.is_null() {
            None
        } else {
            Some(sigmask_ptr.read())
        };

        info!(
            "[sys_ppoll]: fds_ptr {:#x}, nfds {}, timeout:{:?}, sigmask:{:?}",
            fds_ptr, nfds, timeout, sigmask
        );

        let fd_table = self.task.fd_table();
        let mut poll_items = Vec::new();
        let mut fds = Vec::new();
        for i in 0..nfds {
            let fd_ptr = fds_ptr + i * core::mem::size_of::<PollFd>();
            let fd_ptr = UserPtr::<PollFd>::new(fd_ptr);
            let poll_fd = fd_ptr.read();
            debug!("[sys_ppoll]: before poll: poll_fd {:#x?}", poll_fd);
            let file = fd_table.get(poll_fd.fd as usize).ok_or(Errno::EBADF)?;
            let events = poll_fd.events;
            poll_items.push(PpollItem::new(i, events, file));
            fds.push((fd_ptr, poll_fd));
        }
        let ppoll_future = PpollFuture::new(poll_items);

        let mut pcb = self.task.pcb();
        let old_mask = if let Some(mask) = sigmask {
            Some(core::mem::replace(pcb.sig_mask_mut(), mask))
        } else {
            None
        };
        let sig_mask = pcb.sig_mask();
        pcb.set_wake_signal(!sig_mask);

        let res = match TimeLimitedFuture::new(ppoll_future, timeout).await {
            TimeLimitedType::Ok(res) => res,
            TimeLimitedType::TimeOut => {
                debug!("[sys_ppoll]: timeout");
                return Ok(0);
            }
        };

        for (id, result) in res {
            let mut poll_fd = fds[id].1;
            poll_fd.revents |= result;
            fds[id].0.write(poll_fd);
            debug!("[sys_ppoll]: after poll: poll_fd {:#x?}", fds[id].0.read());
        }

        if let Some(old_mask) = old_mask {
            *pcb.sig_mask_mut() = old_mask;
        }
        debug!("[sys_ppoll]: OK!");
        Ok(0)
    }
}

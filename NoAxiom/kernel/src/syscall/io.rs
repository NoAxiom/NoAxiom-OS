use alloc::vec::Vec;
use core::time::Duration;

use include::errno::Errno;
use ksync::assert_no_lock;

use super::{Syscall, SyscallResult};
use crate::{
    include::{
        io::{FdSet, PollEvent, PollFd},
        time::TimeSpec,
    },
    io::{
        ppoll::{PpollFuture, PpollItem},
        pselect::PselectFuture,
    },
    mm::user_ptr::UserPtr,
    signal::{interruptable::interruptable, sig_set::SigSet},
    time::timeout::{TimeLimitedFuture, TimeLimitedType},
};

impl Syscall<'_> {
    pub async fn sys_ppoll(
        &self,
        fds_ptr: usize,
        nfds: usize,
        timeout_ptr: usize,
        sigmask_ptr: usize,
    ) -> SyscallResult {
        let task = self.task;
        let sigmask = UserPtr::<SigSet>::new(sigmask_ptr).try_read().await?;
        let timeout = UserPtr::<TimeSpec>::new(timeout_ptr)
            .try_read()
            .await?
            .map(|x| {
                if !x.is_valid() {
                    error!("[sys_pselect6]: timeout is negative");
                    Err(Errno::EINVAL)
                } else {
                    Ok(Duration::from(x))
                }
            })
            .transpose()?;

        let mut poll_fds = Vec::new();
        let mut fd_ptrs = Vec::new();
        for i in 0..nfds {
            let fd_ptr = fds_ptr + i * core::mem::size_of::<PollFd>();
            let fd_ptr = UserPtr::<PollFd>::new(fd_ptr);
            fd_ptrs.push(fd_ptr);
            poll_fds.push(fd_ptr.read().await?)
        }

        info!(
            "[sys_ppoll]: fds_ptr {:#x}, nfds {}, timeout:{:?}, sigmask:{:?}",
            fds_ptr, nfds, timeout, sigmask
        );

        let fd_table = task.fd_table();
        let mut poll_items = Vec::new();
        let mut fds = Vec::new();
        for i in 0..nfds {
            let poll_fd = poll_fds[i];
            trace!("[sys_ppoll]: before poll: poll_fd {:#x?}", poll_fd);
            let file = fd_table.get(poll_fd.fd as usize).ok_or(Errno::EBADF)?;
            let events = poll_fd.events;
            poll_items.push(PpollItem::new(i, events, file));
            fds.push((fd_ptrs[i], poll_fd));
        }
        drop(fd_table);

        assert_no_lock!();
        let fut = TimeLimitedFuture::new(PpollFuture::new(poll_items), timeout);
        let intable = interruptable(self.task, fut, sigmask, None);
        let res = match intable.await? {
            TimeLimitedType::Ok(res) => res,
            TimeLimitedType::TimeOut => {
                debug!("[sys_ppoll]: timeout");
                return Ok(0);
            }
        };

        let res_len = res.len();
        for (id, result) in res {
            let mut poll_fd = fds[id].1;
            poll_fd.revents |= result;
            fds[id].0.try_write(poll_fd).await?;
            trace!(
                "[sys_ppoll]: after poll: poll_fd {:#x?}",
                fds[id].0.try_read().await?
            );
        }
        Ok(res_len as isize)
    }

    pub async fn sys_pselect6(
        &self,
        nfds: usize,
        readfds_ptr: usize,
        writefds_ptr: usize,
        exceptfds_ptr: usize,
        timeout_ptr: usize,
        sigmask_ptr: usize,
    ) -> SyscallResult {
        info!(
            "[sys_pselect6]: nfds {}, readfds_ptr {:#x}, writefds_ptr {:#x}, exceptfds_ptr {:#x}, timeout_ptr {:#x}, sigmask_ptr {:#x}",
            nfds, readfds_ptr, writefds_ptr, exceptfds_ptr, timeout_ptr, sigmask_ptr
        );

        if (nfds as isize) < 0 {
            error!("[sys_pselect6]: nfds < 0");
            return Err(Errno::EINVAL);
        }

        let timeout = UserPtr::<TimeSpec>::new(timeout_ptr)
            .try_read()
            .await?
            .map(|x| {
                if !x.is_valid() {
                    error!("[sys_pselect6]: timeout is negative");
                    Err(Errno::EINVAL)
                } else {
                    Ok(Duration::from(x))
                }
            })
            .transpose()?;
        let read_fds = UserPtr::<FdSet>::new(readfds_ptr);
        let mut read_fds = read_fds.get_ref_mut().await?;
        let write_fds = UserPtr::<FdSet>::new(writefds_ptr);
        let mut write_fds = write_fds.get_ref_mut().await?;
        let except_fds = UserPtr::<FdSet>::new(exceptfds_ptr);
        let mut except_fds = except_fds.get_ref_mut().await?;
        let sigmask = UserPtr::<SigSet>::new(sigmask_ptr);
        let sigmask = sigmask.try_read().await?;

        info!(
            "[sys_pselect6]: read_fds {:?}, write_fds {:?}, except_fds {:?}, timeout:{:?}, sigmask:{:?}",
            read_fds, write_fds, except_fds, timeout, sigmask
        );

        // collect all poll items
        let fd_table = self.task.fd_table();
        let mut poll_items = Vec::new();
        for fd in 0..nfds as usize {
            let mut events = PollEvent::empty();
            read_fds.as_ref().map(|fds| {
                if fds.is_set(fd) {
                    events.insert(PollEvent::POLLIN)
                }
            });
            write_fds.as_ref().map(|fds| {
                if fds.is_set(fd) {
                    events.insert(PollEvent::POLLOUT)
                }
            });
            // except_fds.as_ref().map(|fds| {
            //     if fds.is_set(fd) {
            //         events.insert(PollEvent::POLLPRI)
            //     }
            // });
            if !events.is_empty() {
                let file = fd_table.get(fd).ok_or(Errno::EBADF)?;
                debug!(
                    "[sys_pselect6] event push fd: {}, file path: {:?}",
                    fd,
                    file.dentry().path()
                );
                poll_items.push(PpollItem::new(fd, events, file));
            }
        }
        drop(fd_table);

        assert_no_lock!();
        let fut = TimeLimitedFuture::new(PselectFuture::new(poll_items), timeout);
        let intable = interruptable(self.task, fut, sigmask, None);
        let res = match intable.await? {
            TimeLimitedType::Ok(res) => Some(res),
            TimeLimitedType::TimeOut => None,
        };

        read_fds.as_mut().map(|fds| fds.clear());
        write_fds.as_mut().map(|fds| fds.clear());
        except_fds.as_mut().map(|fds| fds.clear());

        if res.is_none() {
            debug!("[sys_pselect6]: timeout return Ok(0)");
            return Ok(0);
        }

        let mut ret = 0;
        for (fd, events) in res.unwrap() {
            if events.contains(PollEvent::POLLIN) || events.contains(PollEvent::POLLHUP) {
                read_fds.as_mut().map(|fds| fds.set(fd));
                ret += 1;
            }
            if events.contains(PollEvent::POLLOUT) {
                write_fds.as_mut().map(|fds| fds.set(fd));
                ret += 1;
            }
        }

        Ok(ret)
    }
}

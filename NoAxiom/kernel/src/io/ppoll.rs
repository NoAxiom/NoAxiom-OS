use alloc::{sync::Arc, vec::Vec};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{fs::vfs::basic::file::File, include::io::PollEvent};

pub struct PpollItem {
    id: usize,
    events: PollEvent,
    file: Arc<dyn File>,
}

impl PpollItem {
    pub fn new(id: usize, events: PollEvent, file: Arc<dyn File>) -> Self {
        Self { id, events, file }
    }
}

pub struct PpollFuture {
    fds: Vec<PpollItem>,
}

impl PpollFuture {
    pub fn new(fds: Vec<PpollItem>) -> Self {
        Self { fds }
    }
}

impl Future for PpollFuture {
    type Output = Vec<(usize, PollEvent)>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut result = Vec::new();
        for poll_item in &self.fds {
            let id = poll_item.id;
            let req = &poll_item.events;
            let res = poll_item.file.poll(req, cx.waker().clone());
            if !res.is_empty() {
                result.push((id, res));
            }
        }
        if result.is_empty() {
            debug!("[PpollFuture]: poll result empty, return Pending!");
            Poll::Pending
        } else {
            Poll::Ready(result)
        }
    }
}

use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::time::gettime::get_time_duration;

pub enum TimeLimitedType<T> {
    Ok(T),
    TimeOut,
}

/// A future that will timeout after a certain amount of time().  
///
/// `future`: the target future  
/// `timeout`: the timeout duration  
/// `limit`(don't care): the time limit for the future to finish
///
/// based on poll  
///
/// todo: maybe can based on Interrupt, save waker
///
/// fixme: register on TIME_MANAGER, add current time duration to limit
pub struct TimeLimitedFuture<T: Future> {
    future: Pin<Box<T>>,
    limit: Duration,
}

impl<T: Future> TimeLimitedFuture<T> {
    /// A future that will timeout after a certain amount of time().  
    ///
    /// `future`: the target future  
    /// `timeout`: the timeout duration, None for infinity
    pub fn new(future: T, timeout: Option<Duration>) -> Self {
        Self {
            future: Box::pin(future),
            limit: match timeout {
                Some(t) => t + get_time_duration(),
                None => Duration::MAX,
            },
        }
    }
}

impl<T: Future> Future for TimeLimitedFuture<T> {
    type Output = TimeLimitedType<T::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.future.as_mut().poll(cx) {
            Poll::Ready(res) => return Poll::Ready(TimeLimitedType::Ok(res)),
            Poll::Pending => {
                let now = get_time_duration();
                if now >= self.limit {
                    Poll::Ready(TimeLimitedType::TimeOut)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

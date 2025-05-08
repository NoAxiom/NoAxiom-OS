use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
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
        let limit = match timeout {
            Some(t) => t,
            None => Duration::MAX,
        };
        Self {
            future: Box::pin(future),
            limit,
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

struct GetWakerFuture;

impl Future for GetWakerFuture {
    type Output = Waker;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(cx.waker().clone())
    }
}

/// Get the waker of the current future.
#[allow(unused)]
pub async fn get_current_waker() -> Waker {
    GetWakerFuture.await
}

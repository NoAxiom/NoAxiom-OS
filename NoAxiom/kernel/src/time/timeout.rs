use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    sched::utils::pending_now,
    time::{
        gettime::get_time_duration,
        time_manager::{Timer, TIMER_MANAGER},
    },
};

pub enum TimeLimitedType<T> {
    Ok(T),
    TimeOut,
}

impl<T> TimeLimitedType<T> {
    pub fn map_timeout(self, timeout: T) -> T {
        match self {
            TimeLimitedType::Ok(res) => res,
            TimeLimitedType::TimeOut => timeout,
        }
    }
}

/// A future that will timeout after a certain amount of time().  
///
/// `future`: the target future  
/// `timeout`: the timeout duration  
/// `limit`(don't care): the time limit for the future to finish
///
/// todo: maybe can based on Interrupt, save waker
pub struct TimeLimitedFuture<T: Future> {
    future: Pin<Box<T>>,
    limit: Duration,
    is_pushed: bool,
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
            is_pushed: false,
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
                    if !self.is_pushed {
                        TIMER_MANAGER
                            .add_timer(Timer::new_waker_timer(self.limit, cx.waker().clone()));
                        self.is_pushed = true;
                    }
                    Poll::Pending
                }
            }
        }
    }
}

/// sleep will suspend the task
/// but it can be interrupted, so return the time duration
/// if the result is zero, it indicates the task is woken by sleep event
pub async fn kernel_sleep(interval: Duration) -> Duration {
    let expire = get_time_duration() + interval;
    TimeLimitedFuture::new(pending_now(), Some(interval));
    let now = get_time_duration();
    (expire > now)
        .then_some(expire - now)
        .unwrap_or(Duration::ZERO)
}

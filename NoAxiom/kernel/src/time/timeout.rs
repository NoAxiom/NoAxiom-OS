use alloc::boxed::Box;
use core::{
    future::{pending, Future},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    cpu::current_task,
    signal::interruptable::interruptable,
    time::{
        gettime::get_time_duration,
        timer::{Timer, TIMER_MANAGER},
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

const TIMEOUT_MIN_US: u64 = 500;

impl<T: Future> TimeLimitedFuture<T> {
    /// A future that will timeout after a certain amount of time().  
    ///
    /// `future`: the target future  
    /// `timeout`: the timeout duration, None for infinity
    pub fn new(future: T, timeout: Option<Duration>) -> Self {
        // minimal timeout: 500us
        let timeout = timeout.map(|t| t.max(Duration::from_micros(TIMEOUT_MIN_US)));
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

pub fn busy_wait_timeout(interval: Duration) {
    let time = get_time_duration();
    while get_time_duration() - time < interval {}
}

/// sleep will suspend the task
/// if the result is zero, it indicates the task is woken by sleep event
pub async fn sleep_now(interval: Duration) -> Duration {
    if interval < Duration::from_micros(TIMEOUT_MIN_US) {
        busy_wait_timeout(interval);
        return Duration::ZERO;
    } else {
        sleep_now_interruptable(interval).await
    }
}

pub async fn sleep_now_interruptable(interval: Duration) -> Duration {
    let expire = get_time_duration() + interval;
    // todo add block sleep if the interval is too short
    info!("[sleep] expire: {:?} interval: {:?}", expire, interval);
    TimeLimitedFuture::new(
        interruptable(current_task().unwrap(), pending::<()>(), None, None),
        Some(interval),
    )
    .await;
    let now = get_time_duration();
    if expire > now {
        expire - now
    } else {
        Duration::ZERO
    }
}

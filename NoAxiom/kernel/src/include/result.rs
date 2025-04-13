//! Linux error number: https://man7.org/linux/man-pages/man3/errno.3.html

pub use include::errno::*;

// use alloc::boxed::Box;
// use core::{future::Future, pin::Pin};
// use thiserror::Error;
// sync syscall result
// pub type GeneralRes<T> = core::result::Result<T, Errno>;
// async syscall result
// pub type SysFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
// pub type AsyscallRet<'a> = SysFuture<'a, Result>;

/*

历史遗留问题，我们在进行库解耦之后需要保留原有的import，暂时这么用一下吧

*/

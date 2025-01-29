#[macro_export]
macro_rules! return_errno {
    ($errno:expr $(, $fmt:literal $(, $($arg: tt)+)?)?) => {{
        #[cfg(feature = "dev")]
        {
            let time = crate::time::gettime::get_time::get_time();
            println!("\x1B[93m[{:>1}] {}:{} Errno: {}\x1B[0m", time, file!(), line!(), $errno);
            $(
                println!("\x1B[32m[{:>1}] Reason: {}\x1B[0m", time, format!($fmt $(, $($arg)+)?));
            )?
        }
        return Err($errno);
    }};
}

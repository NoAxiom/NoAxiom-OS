#[macro_export]
macro_rules! return_errno {
    ($errno:expr $(, $fmt:literal $(, $($arg: tt)+)?)?) => {{
        #[cfg(feature = "debug_sig")]
        {
            let time = crate::time::gettime::get_time_duration();
            println!("\x1B[91m[SYSCALL ERROR at {:?}] {}:{} Errno: {}\x1B[0m", time, file!(), line!(), $errno);
            $(
                println!("\x1B[91m[SYSCALL ERROR at {:?}] Reason: {}\x1B[0m", time, format!($fmt $(, $($arg)+)?));
            )?
        }
        return Err($errno);
    }};
}

/// returns pte flags with multiple flag bits
#[macro_export]
macro_rules! pte_flags {
    ($($flag:ident),*) => {
        {
            let mut flags = arch::MappingFlags::empty();
            $(flags |= arch::MappingFlags::$flag;)*
            flags
        }
    };
}

/// returns map permission with multiple flag bits
#[macro_export]
macro_rules! map_permission {
    ($($flag:ident),*) => {
        {
            let mut flags = crate::mm::permission::MapPermission::empty();
            $(flags |= crate::mm::permission::MapPermission::$flag;)*
            flags
        }
    };
}

#[macro_export]
macro_rules! time_statistic {
    ($func_call:expr) => {{
        let start = crate::time::gettime::get_time_duration();
        let result = $func_call;
        let end = crate::time::gettime::get_time_duration();
        let duration = end - start;
        println!(
            "[time_statistic] Executed at {}:{} - Duration: {:?}",
            file!(),
            line!(),
            duration
        );
        result
    }};
    () => {{
        let start = crate::time::gettime::get_time_ms();
        println!(
            "[time_statistic] Executed at {}:{} - {}ms",
            file!(),
            line!(),
            start
        );
    }};
}

#[macro_export]
macro_rules! with_interrupt_on {
    ($func:expr) => {{
        use arch::{Arch, ArchInt};
        let was_enabled = Arch::is_interrupt_enabled();
        Arch::enable_interrupt();
        let result = $func;
        if !was_enabled {
            Arch::disable_interrupt();
        }
        result
    }};
    ($func:block) => {{
        use arch::{Arch, ArchInt};
        let was_enabled = Arch::is_interrupt_enabled();
        Arch::enable_interrupt();
        let result = ($func);
        if !was_enabled {
            Arch::disable_interrupt();
        }
        result
    }};
}

#[macro_export]
macro_rules! with_interrupt_off {
    ($func:expr) => {{
        use arch::{Arch, ArchInt};
        let was_enabled = Arch::is_interrupt_enabled();
        Arch::disable_interrupt();
        let result = $func;
        if was_enabled {
            Arch::enable_interrupt();
        }
        result
    }};
    ($func:block) => {{
        use arch::{Arch, ArchInt};
        let was_enabled = Arch::is_interrupt_enabled();
        Arch::disable_interrupt();
        let result = ($func);
        if was_enabled {
            Arch::enable_interrupt();
        }
        result
    }};
}

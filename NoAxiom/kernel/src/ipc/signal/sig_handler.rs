use crate::include::signal::sig_num::SigNum;

/// terminate the process
fn sig_default_terminate(sig: SigNum) {
    unimplemented!()
}

/// terminate & gen core dump
fn sig_default_terminate_dump(_sig: SigNum) {
    unimplemented!()
}

/// stop the process
fn sig_default_stop(_sig: SigNum) {
    unimplemented!()
}

/// continue the process
fn sig_default_continue(_sig: SigNum) {
    unimplemented!()
}

/// ignore the signal
fn sig_default_ignore(_sig: SigNum) {
    return;
}

/// rt signal default handler
fn sig_default_unimplemented(_sig: SigNum) {
    unimplemented!()
}

pub fn sig_default_handler(signo: i32) {
    let sig: SigNum = signo.into();
    match sig {
        SigNum::INVALID => error!("attempting to handler an Invalid"),
        SigNum::SIGHUP => sig_default_terminate(sig),
        SigNum::SIGINT => sig_default_terminate(sig),
        SigNum::SIGQUIT => sig_default_terminate_dump(sig),
        SigNum::SIGILL => sig_default_terminate_dump(sig),
        SigNum::SIGTRAP => sig_default_terminate_dump(sig),
        SigNum::SIGABRT => sig_default_terminate_dump(sig),
        SigNum::SIGBUS => sig_default_terminate_dump(sig),
        SigNum::SIGFPE => sig_default_terminate_dump(sig),
        SigNum::SIGKILL => sig_default_terminate(sig),
        SigNum::SIGUSR1 => sig_default_terminate(sig),
        SigNum::SIGSEGV => sig_default_terminate_dump(sig),
        SigNum::SIGUSR2 => sig_default_terminate(sig),
        SigNum::SIGPIPE => sig_default_terminate(sig),
        SigNum::SIGALRM => sig_default_terminate(sig),
        SigNum::SIGTERM => sig_default_terminate(sig),
        SigNum::SIGSTKFLT => sig_default_terminate(sig),
        SigNum::SIGCHLD => sig_default_ignore(sig),
        SigNum::SIGCONT => sig_default_continue(sig),
        SigNum::SIGSTOP => sig_default_stop(sig),
        SigNum::SIGTSTP => sig_default_stop(sig),
        SigNum::SIGTTIN => sig_default_stop(sig),
        SigNum::SIGTTOU => sig_default_stop(sig),
        SigNum::SIGURG => sig_default_ignore(sig),
        SigNum::SIGXCPU => sig_default_terminate_dump(sig),
        SigNum::SIGXFSZ => sig_default_terminate_dump(sig),
        SigNum::SIGVTALRM => sig_default_terminate(sig),
        SigNum::SIGPROF => sig_default_terminate(sig),
        SigNum::SIGWINCH => sig_default_ignore(sig),
        SigNum::SIGIO => sig_default_terminate(sig),
        SigNum::SIGPWR => sig_default_terminate(sig),
        SigNum::SIGSYS => sig_default_terminate(sig),
        SigNum::SIGTIMER => sig_default_terminate(sig),
        SigNum::SIGRTMAX => sig_default_terminate(sig),
        _ => sig_default_unimplemented(sig),
    }
}

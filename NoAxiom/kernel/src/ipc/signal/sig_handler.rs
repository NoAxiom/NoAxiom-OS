use crate::include::signal::sig_num::SigNum;

/// terminate the process
fn sig_default_terminate(sig: SigNum) {
    todo!("terminate process with signal {:?}", sig);
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
        SigNum::SIGHUP => sig_default_terminate(sig.clone()),
        SigNum::SIGINT => sig_default_terminate(sig.clone()),
        SigNum::SIGQUIT => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGILL => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGTRAP => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGABRT => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGBUS => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGFPE => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGKILL => sig_default_terminate(sig.clone()),
        SigNum::SIGUSR1 => sig_default_terminate(sig.clone()),
        SigNum::SIGSEGV => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGUSR2 => sig_default_terminate(sig.clone()),
        SigNum::SIGPIPE => sig_default_terminate(sig.clone()),
        SigNum::SIGALRM => sig_default_terminate(sig.clone()),
        SigNum::SIGTERM => sig_default_terminate(sig.clone()),
        SigNum::SIGSTKFLT => sig_default_terminate(sig.clone()),
        SigNum::SIGCHLD => sig_default_ignore(sig.clone()),
        SigNum::SIGCONT => sig_default_continue(sig.clone()),
        SigNum::SIGSTOP => sig_default_stop(sig.clone()),
        SigNum::SIGTSTP => sig_default_stop(sig.clone()),
        SigNum::SIGTTIN => sig_default_stop(sig.clone()),
        SigNum::SIGTTOU => sig_default_stop(sig.clone()),
        SigNum::SIGURG => sig_default_ignore(sig.clone()),
        SigNum::SIGXCPU => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGXFSZ => sig_default_terminate_dump(sig.clone()),
        SigNum::SIGVTALRM => sig_default_terminate(sig.clone()),
        SigNum::SIGPROF => sig_default_terminate(sig.clone()),
        SigNum::SIGWINCH => sig_default_ignore(sig.clone()),
        SigNum::SIGIO => sig_default_terminate(sig.clone()),
        SigNum::SIGPWR => sig_default_terminate(sig.clone()),
        SigNum::SIGSYS => sig_default_terminate(sig.clone()),
        SigNum::SIGTIMER => sig_default_terminate(sig.clone()),
        SigNum::SIGRTMAX => sig_default_terminate(sig.clone()),
        _ => sig_default_unimplemented(sig.clone()),
    }
}

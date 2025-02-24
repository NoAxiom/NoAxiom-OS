use alloc::{string::ToString, vec::Vec};

use crate::{config::fs::INIT_PROC_PATH, fs::path::Path, sched::task::schedule_spawn_new_process};

/// spawn all apps, only used in debug
#[allow(unused)]
pub fn schedule_spawn_all_apps() {
    let mut paths = Vec::new();

    /// Custom tests
    // paths.push("/hello_world");
    // paths.push("/ktest");
    // paths.push("/long_loop");
    // paths.push("/long_loop");
    // paths.push("/long_loop");
    // paths.push("/long_loop");

    /// Official tests
    // fs
    // paths.push("/chdir");
    // paths.push("/yield");
    paths.push("/execve");

    for path in paths {
        schedule_spawn_new_process(Path::from(path.to_string()));
    }
}

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_initproc() {
    info!("[init] spawn initproc");
    schedule_spawn_new_process(Path::from(INIT_PROC_PATH.to_string()));
}

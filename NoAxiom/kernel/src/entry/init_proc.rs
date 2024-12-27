use crate::{config::fs::INIT_PROC_PATH, fs::path::Path, sched::task::schedule_spawn_new_process};

/// spawn all apps, only used in debug
#[allow(unused)]
pub fn schedule_spawn_all_apps() {
    const PATHS: [&str; 1] = [
        // "hello_world",
        // "ktest",
        // "long_loop",
        // "long_loop",
        // "long_loop",
        "process_test",
    ];
    for path in PATHS {
        schedule_spawn_new_process(Path::from(path));
    }
}

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_initproc() {
    info!("[init] spawn initproc");
    schedule_spawn_new_process(Path::from(INIT_PROC_PATH));
}

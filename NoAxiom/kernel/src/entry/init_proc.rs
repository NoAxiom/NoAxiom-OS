use crate::{config::fs::INIT_PROC_NAME, sched::task::schedule_spawn_new_process};

/// spawn all apps, only used in debug
#[allow(unused)]
pub fn schedule_spawn_all_apps() {
    const PATHS: [&str; 6] = [
        "hello_world",
        "ktest",
        "long_loop",
        "long_loop",
        "long_loop",
        "process_test",
    ];
    for path in PATHS.iter() {
        schedule_spawn_new_process(path);
    }
}

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_initproc() {
    info!("[init] spawn initproc");
    schedule_spawn_new_process(INIT_PROC_NAME);
}

use crate::sched::task::schedule_spawn_new_process;

/// spawn all apps, only used in debug
#[allow(unused)]
pub fn schedule_spawn_all_apps() {
    unimplemented!()
    // for i in 0..app_nums() {
    //     info!("[init] spawn app_{}", i);
    //     schedule_spawn_new_process(i);
    // }
}

/// spawn init process
#[allow(unused)]
pub fn spawn_initproc() {
    info!("[init] spawn initproc");
    schedule_spawn_new_process();
}

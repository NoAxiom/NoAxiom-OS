use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    fs::path::Path,
    include::fs::InodeMode,
    mm::memory_set::MemorySet,
    sched::spawn::{spawn_ktask, spawn_utask},
    task::Task,
};

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_with_path() {
    const INIT_PROC_PATH: &str = "init_proc";
    info!("[init] spawn initproc with path = {}", INIT_PROC_PATH);
    spawn_ktask(async move {
        // new process must be EXECUTABLE file, not directory
        let path = Path::from_or_create(INIT_PROC_PATH.to_string(), InodeMode::FILE).await;
        let elf = MemorySet::load_from_path(path.clone()).await;
        let task = Task::new_process(elf);
        spawn_utask(task);
    });
}

#[allow(unused)]
pub fn schedule_spawn_with_kernel_app() {
    macro_rules! use_app {
        ($name:literal) => {
            extern "C" {
                #[link_name = concat!($name, "_start")]
                fn app_start();
                #[link_name = concat!($name, "_end")]
                fn app_end();
            }
            const INIT_PROC_NAME: &str = $name;
            #[cfg(feature = "glibc")]
            const INIT_PROC_PATH: &str = concat!("/glibc/", $name);
            #[cfg(not(feature = "glibc"))]
            const INIT_PROC_PATH: &str = concat!("/musl/", $name);
        };
    }
    use_app!("run_busybox");
    info!(
        "[init] spawn initproc with app name = {}, path = {}",
        INIT_PROC_NAME, INIT_PROC_PATH
    );
    spawn_ktask(async move {
        let start = app_start as usize;
        let end = app_end as usize;
        let size = end - start;
        debug!(
            "[kernel_app] start: {:#x}, end: {:#x}, size: {}",
            start, end, size
        );
        let file_data = Vec::from(unsafe { core::slice::from_raw_parts(start as *const u8, size) });
        let elf = MemorySet::load_from_vec(file_data).await;
        let task = Task::new_process(elf);
        let path = Path::from_or_create(INIT_PROC_PATH.to_string(), InodeMode::FILE).await;
        let path = path.from_cd("..").expect("directory not found");
        let mut guard = task.cwd();
        *guard = path;
        drop(guard);
        spawn_utask(task);
    });
}

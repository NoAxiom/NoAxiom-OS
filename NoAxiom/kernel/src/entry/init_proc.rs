use alloc::{string::ToString, vec::Vec};

use config::fs::ROOT_NAME;

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
        let elf = MemorySet::load_from_path(path.clone()).await.unwrap();
        let task = Task::new_process(elf).await;
        spawn_utask(task);
    });
}

macro_rules! use_app {
    ($name:literal) => {
        extern "C" {
            #[link_name = concat!($name, "_start")]
            fn app_start();
            #[link_name = concat!($name, "_end")]
            fn app_end();
        }
        pub const INIT_PROC_NAME: &str = $name;
    };
}
#[cfg(feature = "busybox")]
use_app!("run_busybox");

#[allow(unused)]
pub fn schedule_spawn_with_kernel_app() {
    println!(
        "[kernel] spawn initproc with app name = {}, path = {}",
        INIT_PROC_NAME, ROOT_NAME
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
        let elf = MemorySet::load_from_vec(file_data).await.unwrap();
        let task = Task::new_process(elf).await;
        spawn_utask(task);
    });
}

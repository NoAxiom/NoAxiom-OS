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
    println!(
        "[kernel] initproc = {}, path = {}/",
        INIT_PROC_NAME, ROOT_NAME
    );
    spawn_ktask(async move {
        let path_str = format!("{}/{}", ROOT_NAME, INIT_PROC_NAME);
        let path = Path::from_or_create(path_str, InodeMode::FILE).await;
        let file = path.dentry().open().unwrap();
        let content = get_file();
        file.base_write(0, content).await.unwrap();
        #[cfg(feature = "debug_sig")]
        {
            let mut read_buf = vec![0u8; content.len()];
            let read_size = file.base_read(0, &mut read_buf).await.unwrap();
            assert_eq!(read_buf, content);
        }
        let elf_file = path.dentry().open().unwrap();
        let elf = MemorySet::load_elf(&elf_file).await.unwrap();
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
#[cfg(feature = "runtests")]
use_app!("run_tests");

// #[allow(unused)]
// pub fn schedule_spawn_with_kernel_app() {
//     println!(
//         "[kernel] spawn initproc with app name = {}, path = {}",
//         INIT_PROC_NAME, ROOT_NAME
//     );
//     spawn_ktask(async move {
//         let file_data = get_file();
//         let elf = MemorySet::load_from_vec(file_data).await.unwrap();
//         let task = Task::new_process(elf).await;
//         spawn_utask(task);
//     });
// }

fn get_file<'a>() -> &'a [u8] {
    let start = app_start as usize;
    let end = app_end as usize;
    let size = end - start;
    debug!(
        "[kernel_app] start: {:#x}, end: {:#x}, size: {}",
        start, end, size
    );
    unsafe { core::slice::from_raw_parts(start as *const u8, size) }
}

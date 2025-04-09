use alloc::{string::ToString, vec::Vec};

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
        };
    }
    use_app!("run_busybox");
    info!("[init] spawn initproc with app name = {}", INIT_PROC_NAME);
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
        spawn_utask(task);
    });
}

/*

our arrangement:
.quad app_run_busybox_start
.quad app_run_busybox_end
.quad app_runtests_start
.quad app_runtests_end

#[allow(unused)]
pub async fn get_busybox_data() -> Option<Arc<dyn File>> {
    extern "C" {
        fn initproc_entry();
        fn initproc_tail();
    }
    let entry = initproc_entry as usize;
    let tail = initproc_tail as usize;
    let siz = tail - entry;
    let initproc = unsafe { core::slice::from_raw_parts(entry as *const u8, siz) };
    let path = AbsolutePath::from_str("/initproc");
    let inode = open(
        path,
        OpenFlags::O_CREATE | OpenFlags::O_WRONLY,
        CreateMode::empty(),
    )
    .expect("initproc create failed!");
    inode
        .write(initproc.to_owned().len(), &initproc.to_owned())
        .await;
    Some(inode)
}

*/

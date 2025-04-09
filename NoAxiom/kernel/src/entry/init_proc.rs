use alloc::string::ToString;

use crate::{
    fs::path::Path,
    include::fs::InodeMode,
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
        let task = Task::new_process(path).await;
        spawn_utask(task);
    });
}

#[allow(unused)]
pub fn schedule_spawn_with_kernel_app() {
    const INIT_PROC_NAME: &str = "run_busybox";
    info!("[init] spawn initproc with app name = {}", INIT_PROC_NAME);
    spawn_ktask(async move {
        // push your code here
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

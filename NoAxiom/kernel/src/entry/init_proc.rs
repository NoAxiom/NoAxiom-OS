use crate::{
    fs::path::Path,
    include::fs::{FileFlags, InodeMode},
    mm::memory_set::MemorySet,
    sched::spawn::{spawn_ktask, spawn_utask},
    task::Task,
};

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_with_path() {
    println!("[kernel] INIT_PROC: {}", INIT_PROC_NAME);
    spawn_ktask(async move {
        let path_str = format!("/{}", INIT_PROC_NAME);
        let path = Path::from_or_create(path_str, InodeMode::FILE)
            .await
            .unwrap();
        let file = path.dentry().open(&FileFlags::O_RDWR).unwrap();
        let content = get_content(INIT_PROC_NAME);
        file.write_at(0, content).await.unwrap();
        #[cfg(feature = "debug_sig")]
        {
            let mut read_buf = vec![0u8; content.len()];
            let read_size = file.read_at(0, &mut read_buf).await.unwrap();
            assert_eq!(read_buf, content);
        }
        let elf_file = path.dentry().open(&FileFlags::O_RDWR).unwrap();
        let elf = MemorySet::load_elf(&elf_file).await.unwrap();
        let task = Task::new_init_process(elf).await;
        spawn_utask(task);
    });
}

macro_rules! use_apps {
    ($($name:literal),+ $(,)?) => {
        $(
            paste::paste! {
            extern "C" {
                #[link_name = concat!($name, "_start")]
                fn [<$name _start>]();
                #[link_name = concat!($name, "_end")]
                fn [<$name _end>]();
            }

            fn [<$name _get_content>]() -> &'static [u8] {
                let start = [<$name _start>] as usize;
                let end = [<$name _end>] as usize;
                let size = end - start;
                debug!(
                    "[kernel_app] start: {:#x}, end: {:#x}, size: {}",
                    start, end, size
                );
                unsafe { core::slice::from_raw_parts(start as *const u8, size) }
            }
        }
        )+
        pub const FINAL_TESTS: &[&str] = &[$($name),+];
    };
}

macro_rules! gen_get_content {
    ($($name:literal),+ $(,)?) => {
        fn get_content(name: &str) -> &'static [u8] {
            match name {
                $(
                    $name => paste::paste! { [<$name _get_content>]() },
                )+
                _ => panic!("Unknown app name: {}", name),
            }
        }
    };
}

#[cfg(feature = "busybox")]
pub const INIT_PROC_NAME: &'static str = "run_busybox";

#[cfg(feature = "runtests")]
pub const INIT_PROC_NAME: &'static str = "run_tests";

use_apps!("run_busybox", "run_tests",);
gen_get_content!("run_busybox", "run_tests");

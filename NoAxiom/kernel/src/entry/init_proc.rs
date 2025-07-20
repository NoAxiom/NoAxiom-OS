use crate::{
    fs::path::Path,
    include::fs::InodeMode,
    mm::memory_set::MemorySet,
    sched::spawn::{spawn_ktask, spawn_utask},
    task::Task,
};

// just for simple, todo: use macro
fn get_content(name: &str) -> &'static [u8] {
    match name {
        #[cfg(feature = "busybox")]
        "run_busybox" => run_busybox_get_content(),
        #[cfg(feature = "runtests")]
        "run_tests" => run_tests_get_content(),
        #[cfg(feature = "final_test")]
        "interrupts_test_1_rv_musl" => interrupts_test_1_rv_musl_get_content(),
        #[cfg(feature = "final_test")]
        "interrupts_test_2_rv_musl" => interrupts_test_2_rv_musl_get_content(),
        _ => panic!("Unknown app name: {}", name),
    }
}

/// spawn init process
#[allow(unused)]
pub fn schedule_spawn_with_path() {
    println!("[kernel] INIT_PROC: {}", INIT_PROC_NAME);
    spawn_ktask(async move {
        let path_str = format!("/{}", INIT_PROC_NAME);
        let path = Path::from_or_create(path_str, InodeMode::FILE)
            .await
            .unwrap();
        let file = path.dentry().open().unwrap();
        let content = get_content(INIT_PROC_NAME);
        file.write_at(0, content).await.unwrap();
        #[cfg(feature = "debug_sig")]
        {
            let mut read_buf = vec![0u8; content.len()];
            let read_size = file.read_at(0, &mut read_buf).await.unwrap();
            assert_eq!(read_buf, content);
        }
        let elf_file = path.dentry().open().unwrap();
        let elf = MemorySet::load_elf(&elf_file).await.unwrap();
        let task = Task::new_init_process(elf).await;
        spawn_utask(task);

        #[cfg(feature = "final_test")]
        {
            const LIBS: &[&str] = &["/glibc", "/musl"];
            for lib in LIBS {
                for name in FINAL_TESTS {
                    let path_str = format!("{}/{}", *lib, *name);
                    let path = Path::from_or_create(path_str, InodeMode::FILE)
                        .await
                        .unwrap();
                    let file = path.dentry().open().unwrap();
                    let content = get_content(*name);
                    file.write_at(0, content).await.unwrap();
                }
            }
        }
    });
}

macro_rules! use_app {
    ($name:literal) => {
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
    };
    ($name:literal,"init_proc") => {
        use_app!($name);
        pub const INIT_PROC_NAME: &str = $name;
    };
}

macro_rules! use_apps {
    ($($name:literal),+ $(,)?) => {
        $(
            use_app!($name);
        )+
        pub const FINAL_TESTS: &[&str] = &[$($name),+];
    };
}

#[cfg(feature = "busybox")]
use_app!("run_busybox", "init_proc");

#[cfg(feature = "runtests")]
use_app!("run_tests", "init_proc");

#[cfg(feature = "final_test")]
use_apps!("interrupts_test_1_rv_musl", "interrupts_test_2_rv_musl");

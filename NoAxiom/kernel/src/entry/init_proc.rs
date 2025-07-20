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
            for name in FINAL_TESTS {
                use alloc::string::ToString as _;

                #[cfg(target_arch = "riscv64")]
                let arch = "rv";
                #[cfg(target_arch = "loongarch64")]
                let arch = "la";

                let musl_end = format!("_{}_musl", arch);
                let glibc_end = format!("_{}_glibc", arch);

                let path_str = if (*name).ends_with(&musl_end) {
                    format!("/musl/{}", *name)
                } else if (*name).ends_with(&glibc_end) {
                    format!("/glibc/{}", *name)
                } else {
                    continue;
                };

                let path_str = if path_str.ends_with(&musl_end) {
                    path_str.trim_end_matches(&musl_end)
                } else if path_str.ends_with(&glibc_end) {
                    path_str.trim_end_matches(&glibc_end)
                } else {
                    unreachable!("Unknown app name: {}", name);
                };

                let path = Path::from_or_create(path_str.to_string(), InodeMode::FILE)
                    .await
                    .unwrap();
                let file = path.dentry().open().unwrap();
                let content = get_content(*name);
                file.write_at(0, content).await.unwrap();
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
use_app!("run_busybox", "init_proc");

#[cfg(feature = "runtests")]
use_app!("run_tests", "init_proc");

#[cfg(feature = "final_test")]
use_apps!(
    "interrupts_test_1_rv_musl",
    "interrupts_test_2_rv_musl",
    "copy_file_range_test_1_rv_musl",
    "copy_file_range_test_2_rv_musl",
    "copy_file_range_test_3_rv_musl",
    "copy_file_range_test_4_rv_musl"
);

gen_get_content!(
    "run_tests",
    "interrupts_test_1_rv_musl",
    "interrupts_test_2_rv_musl",
    "copy_file_range_test_1_rv_musl",
    "copy_file_range_test_2_rv_musl",
    "copy_file_range_test_3_rv_musl",
    "copy_file_range_test_4_rv_musl",
);

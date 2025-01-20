#![no_std]
#![no_main]

use userlib::{
    println,
    syscall::{sys_exec, sys_fork, sys_times, sys_yield, TMS},
};

#[no_mangle]
fn main(argc: isize, argv: *const *const u8) -> isize {
    println!("process_test: Hello, world!");

    let mut tms_start = TMS::new();
    sys_times(&mut tms_start);

    sys_exec("ktest\0");
    // let pid = sys_fork();
    // if pid == 0 {
    //     println!("child process0, exec ktest");
    //     sys_exec("ktest\0");
    // }

    // let pid = sys_fork();
    // if pid == 0 {
    //     let mut tms = TMS::new();
    //     sys_times(&mut tms);
    //     println!("[child process1] tms: {:?}", tms);
    // } else {
    //     println!("[parent process1] do nothing");
    // }

    // let pid = sys_fork();
    // if pid == 0 {
    //     println!("[child process2] do nothing");
    // } else {
    //     println!("[parent process2] yield start");
    //     for i in 0..10 {
    //         println!("[parent process2] yield: {}", i);
    //         sys_yield();
    //     }
    //     println!("[parent process2] yield done");
    // }

    // // let pid = sys_fork();
    // // if pid == 0 {
    // //     println!("[child process3] do nothing");
    // // } else {
    // //     println!("[parent process3] do nothing");
    // //     // println!("[parent process3] exec ktest");
    // //     // sys_exec("ktest\0");
    // //     // println!("ERROR!!! unreachable: parent process3");
    // // }

    // let mut tms_end = TMS::new();
    // sys_times(&mut tms_end);
    // println!(
    //     "[main] tms_start: {:?}, tms_end: {:?}, gap: {}",
    //     tms_start,
    //     tms_end,
    //     tms_end.0 - tms_start.0
    // );
    0
}

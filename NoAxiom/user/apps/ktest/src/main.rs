#![no_std]
#![no_main]

use userlib::println;

#[no_mangle]
fn main(argc: usize, argv: &[&str], envp: &[&str]) -> isize {
    println!("[ktest] EXECVE test start");
    println!("[ktest] argc: {}", argc);
    for i in 0..argc {
        println!("[ktest] argv[{}]: {}", i, argv[i]);
    }
    for i in 0..envp.len() {
        println!("[ktest] envp[{}]: {}", i, envp[i]);
    }

    // println!(
    //     "[ktest] argc: {}, argv: {}, envp: {}",
    //     argc, argv as usize, envp as usize
    // );
    // for i in 0..argc {
    //     let arg = unsafe { argv.offset(i) };
    //     println!(
    //         "[ktest] argv[{}]: reading at addr: {}",
    //         i,
    //         unsafe { *(arg as *const usize) } as usize
    //     );
    //     let arg = unsafe { *arg };
    //     print!("[ktest] argv[{}] ", i);
    //     let mut j = 0;
    //     loop {
    //         let c = unsafe { *arg.offset(j) };
    //         if c == 0 {
    //             break;
    //         }
    //         print!("{}", c as char);
    //         j += 1;
    //     }
    //     print!("\n");
    // }
    0
}

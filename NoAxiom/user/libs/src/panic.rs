use crate::{println, syscall::sys_exit};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let err = info.message().unwrap();
    if let Some(location) = info.location() {
        println!(
            "[user] Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            err
        );
    } else {
        println!("[user] Panicked: {}", err);
    }
    sys_exit(-1)
}

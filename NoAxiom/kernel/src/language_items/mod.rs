use core::panic;

#[panic_handler]
fn panic(_info: &panic::PanicInfo) -> ! {
    loop {}
}

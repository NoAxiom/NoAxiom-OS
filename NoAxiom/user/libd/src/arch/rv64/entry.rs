use core::arch::asm;

use crate::entry::entry;

#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start() -> ! {
    let mut argc: usize;
    let mut argv: usize;
    unsafe {
        asm! {
            "ld a0, 0(sp)",
            "ld a1, 8(sp)",
            out("a0") argc,
            out("a1") argv,
        }
    }
    entry(argc, argv)
}

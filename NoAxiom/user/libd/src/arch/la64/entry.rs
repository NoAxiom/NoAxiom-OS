use core::arch::asm;

use crate::entry::entry;

#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start() -> ! {
    let argc: usize;
    let argv: usize;
    unsafe {
        asm!(
            "ld.d $a0, $sp, 0",
            "ld.d $a1, $sp, 8",
            out("$a0") argc,
            out("$a1") argv
        );
    }
    entry(argc, argv)
}

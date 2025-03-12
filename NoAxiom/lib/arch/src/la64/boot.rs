use core::arch::asm;

extern "C" {
    fn _boot_hart_init();
    fn _other_hart_init();
}

pub unsafe extern "C" fn _entry() -> ! {
    asm!("", options(noreturn))
}

pub unsafe extern "C" fn _entry_other_hart() -> ! {
    asm!("", options(noreturn))
}

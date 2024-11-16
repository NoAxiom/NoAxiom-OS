core::arch::global_asm!(include_str!("./entry.asm"));

pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });
}

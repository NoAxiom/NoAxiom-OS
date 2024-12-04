pub fn bss_init() {
    extern "C" {
        fn skstack();
        fn ebss();
    }
    (skstack as usize..ebss as usize).for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });
}

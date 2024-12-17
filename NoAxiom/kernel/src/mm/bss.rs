/// This function is called only once during booting.
/// DO NOT try to modify any global / unstacked variable before this function!
/// NOTE THAT this function will not clear any data on the kernel stack,
/// since the beginning address is `ekstack`.
pub fn bss_init() {
    extern "C" {
        fn ekstack();
        fn ebss();
    }
    (ekstack as usize..ebss as usize).for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });
}

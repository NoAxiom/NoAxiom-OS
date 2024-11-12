#[macro_export]
macro_rules! hartid {
    () => { {
        let hartid: usize;
        unsafe {core::arch::asm!("mv {}, tp", out(reg) hartid)}
        hartid
    }};
}

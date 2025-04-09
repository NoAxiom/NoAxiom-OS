use core::arch::global_asm;

global_asm!(include_str!("syscall.S"));
extern "C" {
    pub fn __syscall(
        id: usize,
        args0: usize,
        args1: usize,
        args2: usize,
        args3: usize,
        args4: usize,
        args5: usize,
    ) -> isize;
}

pub fn syscall(id: usize, args: [usize; 6]) -> isize {
    #[cfg(target_arch = "loongarch64")]
    unsafe {
        __syscall(id, args[0], args[1], args[2], args[3], args[4], args[5])
    }
}

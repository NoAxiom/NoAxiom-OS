use core::arch::asm;

// 用户态系统调用的接口
#[inline(always)]
pub fn syscall(id: usize, args: [usize; 6]) -> isize {
    let mut ret: isize;
    // 通过汇编指令描述了具体用哪些寄存器来保存参数和返回值
    // 返回内核态后，通过系统调用的请求从寄存器中取得相应的值并执行相应系统调用
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x17") id
        );
    }
    ret
}

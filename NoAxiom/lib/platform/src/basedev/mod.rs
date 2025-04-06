#[cfg(all(target_arch = "loongarch64", feature = "board"))]
mod loongarch64_board;
#[cfg(all(target_arch = "loongarch64", feature = "qemu"))]
mod loongarch64_qemu;
#[cfg(all(target_arch = "riscv64", feature = "board"))]
mod riscv64_board;
#[cfg(all(target_arch = "riscv64", feature = "qemu"))]
mod riscv64_qemu;

#[cfg(all(target_arch = "loongarch64", feature = "board"))]
mod loongarch64_board;
#[cfg(all(target_arch = "loongarch64", feature = "qemu"))]
pub type Base = loongarch64_qemu::Base;
#[cfg(all(target_arch = "riscv64", feature = "board"))]
mod riscv64_board;
#[cfg(all(target_arch = "riscv64", feature = "qemu"))]
pub type Base = riscv64_qemu::Base;

pub trait BaseFu {
    fn putchar(c: usize);
    fn getchar() -> usize;
    fn shutdown() -> !;
}

pub fn putchar(c: usize) {
    Base::putchar(c);
}
pub fn getchar() -> usize {
    Base::getchar()
}
pub fn shutdown() -> ! {
    Base::shutdown()
}

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
    fn putchar(c: u8);
    fn getchar() -> u8;
    fn shutdown() -> !;
}

pub fn putchar(c: u8) {
    Base::putchar(c);
}
pub fn getchar() -> u8 {
    Base::getchar()
}
pub fn shutdown() -> ! {
    Base::shutdown()
}

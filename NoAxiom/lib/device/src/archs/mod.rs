#[cfg(target_arch = "loongarch64")]
mod la;
#[cfg(target_arch = "riscv64")]
mod rv;

#[cfg(target_arch = "loongarch64")]
pub use la::*;
#[cfg(target_arch = "riscv64")]
pub use rv::*;

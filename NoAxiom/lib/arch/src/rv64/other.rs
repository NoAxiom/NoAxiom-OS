use super::RV64;
use crate::ArchInfo;

impl ArchInfo for RV64 {
    const ARCH_NAME: &'static str = "riscv64";
}

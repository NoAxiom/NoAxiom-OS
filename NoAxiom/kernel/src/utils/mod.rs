//! utility functions

use crate::config::mm::{KERNEL_ADDR_OFFSET, KERNEL_PAGENUM_MASK};

/// signed extend for number without 64/32 bits width
pub fn signed_extend(num: usize, width: usize) -> usize {
    if num & (1 << (width - 1)) != 0 {
        num | (!((1 << width) - 1))
    } else {
        num
    }
}

/// translate a raw usize type kernel virt address into phys address
pub fn kernel_va_to_pa(virt: usize) -> usize {
    info!(
        "kernel_va_to_pa: {:#x}, {:#x}, {:#x}",
        virt,
        virt & KERNEL_ADDR_OFFSET,
        KERNEL_ADDR_OFFSET
    );
    assert!(
        (virt & KERNEL_ADDR_OFFSET) == KERNEL_ADDR_OFFSET,
        "invalid kernel virt address"
    );
    virt & !KERNEL_ADDR_OFFSET
}

/// get current pc
#[inline(always)]
#[allow(unused)]
pub fn current_pc() -> usize {
    let pc: usize;
    unsafe { core::arch::asm!("auipc {}, 0", out(reg) pc) }
    pc
}

pub fn kernel_vpn_to_ppn(vpn: usize) -> usize {
    vpn & !KERNEL_PAGENUM_MASK
}

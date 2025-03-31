#![allow(unused)]

use alloc::borrow::ToOwned;

use riscv::{asm::sfence_vma_all, register::satp};

use super::RV64;
use crate::ArchInt;

/// check if interrupt is enabled
#[inline(always)]
pub fn is_interrupt_enabled() -> bool {
    riscv::register::sstatus::read().sie()
}

/// set int disabled
#[inline(always)]
pub fn disable_interrupt() {
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
}

/// set int enabled
#[inline(always)]
pub fn enable_interrupt() {
    unsafe {
        riscv::register::sstatus::set_sie();
    }
}

/// set external int enabled
#[inline(always)]
pub fn enable_external_interrupt() {
    unsafe {
        riscv::register::sie::set_sext();
    }
}

/// check if external interrupt is enabled
#[inline(always)]
pub fn is_external_interrupt_enabled() -> bool {
    riscv::register::sie::read().sext() && is_interrupt_enabled()
}

/// set external int disabled
#[inline(always)]
pub fn disable_external_interrupt() {
    unsafe {
        riscv::register::sie::clear_sext();
    }
}

/// set soft int enabled
#[inline(always)]
pub fn enable_software_interrupt() {
    unsafe {
        riscv::register::sie::set_ssoft();
    }
}

/// set supervisor timer int enabled
#[inline(always)]
pub fn enable_stimer_interrupt() {
    unsafe {
        riscv::register::sie::set_stimer();
    }
}

/// permit supervisor user memory access
#[inline(always)]
pub fn enable_user_memory_access() {
    unsafe {
        riscv::register::sstatus::set_sum();
    }
}

/// clear supervisor user memory access
#[inline(always)]
pub fn disable_user_memory_access() {
    unsafe {
        riscv::register::sstatus::clear_sum();
    }
}

impl ArchInt for RV64 {
    // check if global interrupt is enabled
    #[inline(always)]
    fn is_interrupt_enabled() -> bool {
        is_interrupt_enabled()
    }

    // global interrupt
    #[inline(always)]
    fn disable_interrupt() {
        disable_interrupt();
    }
    #[inline(always)]
    fn enable_interrupt() {
        enable_interrupt();
    }

    // external interrupt
    #[inline(always)]
    fn enable_external_interrupt() {
        enable_external_interrupt();
    }
    #[inline(always)]
    fn disable_external_interrupt() {
        disable_external_interrupt();
    }
    #[inline(always)]
    fn is_external_interrupt_enabled() -> bool {
        is_external_interrupt_enabled()
    }

    // soft / timer interrupt
    #[inline(always)]
    fn enable_software_interrupt() {
        enable_software_interrupt();
    }
    #[inline(always)]
    fn enable_timer_interrupt() {
        enable_stimer_interrupt();
    }

    // user memory access
    #[inline(always)]
    fn enable_user_memory_access() {
        enable_user_memory_access();
    }
    #[inline(always)]
    fn disable_user_memory_access() {
        disable_user_memory_access();
    }
}

#![allow(unused)]

use alloc::borrow::ToOwned;

use riscv::{asm::sfence_vma_all, register::satp};

/// check if interrupt is enabled
pub fn is_interrupt_enabled() -> bool {
    riscv::register::sstatus::read().sie()
}

/// set int disabled
pub fn disable_global_interrupt() {
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
}

/// set int enabled
pub fn enable_global_interrupt() {
    unsafe {
        riscv::register::sstatus::set_sie();
    }
}

/// set external int enabled
pub fn enable_external_interrupt() {
    unsafe {
        riscv::register::sie::set_sext();
    }
}

/// check if external interrupt is enabled
pub fn is_external_interrupt_enabled() -> bool {
    riscv::register::sie::read().sext() && is_interrupt_enabled()
}

/// set external int disabled
pub fn disable_external_interrupt() {
    unsafe {
        riscv::register::sie::clear_sext();
    }
}

/// set soft int enabled
pub fn enable_software_interrupt() {
    unsafe {
        riscv::register::sie::set_ssoft();
    }
}

/// set supervisor timer int enabled
pub fn enable_stimer_interrupt() {
    unsafe {
        riscv::register::sie::set_stimer();
    }
}

/// permit supervisor user memory access
pub fn enable_user_memory_access() {
    unsafe {
        riscv::register::sstatus::set_sum();
    }
}

/// clear supervisor user memory access
pub fn disable_user_memory_access() {
    unsafe {
        riscv::register::sstatus::clear_sum();
    }
}

use riscv::{asm::sfence_vma_all, register::satp};

/// 检查全局中断是否开启
pub fn is_interrupt_enable() -> bool {
    riscv::register::sstatus::read().sie()
}

/// 关闭全局中断
pub fn interrupt_disable() {
    unsafe {
        riscv::register::sstatus::clear_sie();
    }
}

/// 开启全局中断
pub fn interrupt_enable() {
    unsafe {
        riscv::register::sstatus::set_sie();
    }
}

/// 开启外部中断
pub fn external_interrupt_enable() {
    unsafe {
        riscv::register::sie::set_sext();
    }
}

/// 开启软件中断
pub fn software_interrupt_enable() {
    unsafe {
        riscv::register::sie::set_ssoft();
    }
}

/// 关闭外部中断
pub fn external_interrupt_disable() {
    unsafe {
        riscv::register::sie::clear_sext();
    }
}

/// 开启时钟中断
pub fn enable_stimer_interrupt() {
    unsafe {
        riscv::register::sie::set_stimer();
    }
}

/// 激活页表模式
pub fn activate_paging_mode(root_ppn: usize) {
    unsafe {
        sfence_vma_all();
        satp::set(satp::Mode::Sv39, 0, root_ppn);
        sfence_vma_all();
    }
}

/// Permit Supervisor User Memory access
pub fn enable_visit_user_memory() {
    unsafe {
        riscv::register::sstatus::set_sum();
    }
}

/// Permit Supervisor User Memory access
pub fn disable_visit_user_memory() {
    unsafe {
        riscv::register::sstatus::clear_sum();
    }
}

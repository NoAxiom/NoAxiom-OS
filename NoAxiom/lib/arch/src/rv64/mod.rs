mod asm;
mod interrupt;
mod register;
mod sbi;
mod trap_cx;

use asm::*;
use interrupt::*;
use register::*;
use riscv::asm::sfence_vma_all;
use sbi::*;
use sbi_rt::{
    hart_start,
    legacy::{clear_ipi, console_getchar, console_putchar, set_timer, shutdown},
    send_ipi,
};

use crate::{
    ArchAsm, ArchInfo, ArchInt, ArchMemory, ArchSbi, ArchTime, ArchTrap, ArchType, FullVirtArch,
};

pub struct RV64;

impl ArchInt for RV64 {
    // check if global interrupt is enabled
    #[inline(always)]
    fn is_interrupt_enabled() -> bool {
        is_interrupt_enabled()
    }

    // global interrupt
    #[inline(always)]
    fn disable_global_interrupt() {
        disable_global_interrupt();
    }
    #[inline(always)]
    fn enable_global_interrupt() {
        enable_global_interrupt();
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
    fn enable_stimer_interrupt() {
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

impl ArchAsm for RV64 {
    #[inline(always)]
    fn get_hartid() -> usize {
        get_hartid()
    }
    #[inline(always)]
    fn set_idle() {
        set_idle();
    }
}

impl ArchType for RV64 {
    type Trap = riscv::register::scause::Trap;
    type Interrupt = riscv::register::scause::Interrupt;
    type Exception = riscv::register::scause::Exception;
    type TrapContext = trap_cx::TrapContext;
}

impl ArchSbi for RV64 {
    // write in console
    fn console_putchar(c: usize) {
        console_putchar(c);
    }
    // read in console
    fn console_getchar() -> usize {
        console_getchar()
    }
    // send ipi
    fn send_ipi(hartid: usize) {
        trigger_ipi(hartid);
    }
    // clear ipi
    fn clear_ipi() {
        clear_ipi();
    }
    // shutdown
    fn shutdown() -> ! {
        shutdown()
    }
    // hart start
    fn hart_start(hartid: usize, start_addr: usize, opaque: usize) {
        let x = hart_start(hartid, start_addr, opaque);
        if x.is_err() {
            panic!("hart_start failed");
        }
    }
}

impl ArchMemory for RV64 {
    // flush all TLB
    #[inline(always)]
    fn tlb_flush() {
        sfence_vma_all();
    }
    // update page table base address
    #[inline(always)]
    fn update_pagetable(bits: usize) {
        update_pagetable(bits);
    }
}

impl ArchTrap for RV64 {
    #[inline(always)]
    fn set_trap_entry(addr: usize) {
        set_trap_entry(addr);
    }
    fn read_trap_cause() -> Self::Trap {
        scause::read().cause()
    }
    fn read_trap_value() -> usize {
        stval::read()
    }
    fn read_trap_pc() -> usize {
        sepc::read()
    }
}

impl ArchTime for RV64 {
    #[inline(always)]
    fn get_time() -> usize {
        time::read()
    }
    fn set_timer(time_value: u64) -> usize {
        set_timer(time_value)
    }
}

impl ArchInfo for RV64 {
    const ARCH_NAME: &'static str = "riscv64";
}

impl FullVirtArch for RV64 {}

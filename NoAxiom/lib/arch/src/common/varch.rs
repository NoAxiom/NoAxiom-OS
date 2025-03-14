//! trait bound list:
//! - [`FullVirtArch`] impl all traits below
//! - [`ArchInt`]
//! - [`ArchAsm`]
//! - [`ArchSbi`]
//! - [`ArchTrap`]
//! - [`ArchTime`]
//! - [`ArchInfo`]
//! - [`ArchMemory`]

use core::ops::{Index, IndexMut};

use super::{ArchPageTable, MappingFlags};
use crate::{TrapArgs, TrapType};

/// interrupt related arch trait
pub trait ArchInt {
    // global interrupt
    fn is_interrupt_enabled() -> bool;
    fn disable_global_interrupt();
    fn enable_global_interrupt();

    // external interrupt
    fn enable_external_interrupt();
    fn disable_external_interrupt();
    fn is_external_interrupt_enabled() -> bool;

    // soft / timer interrupt
    fn enable_software_interrupt();
    fn enable_stimer_interrupt();

    // user memory access
    fn enable_user_memory_access();
    fn disable_user_memory_access();
}

/// hart related arch trait
pub trait ArchAsm {
    fn get_hartid() -> usize;
    fn set_idle();
    fn current_pc() -> usize;
}

/// sbi related arch trait
pub trait ArchSbi {
    fn console_putchar(_c: usize);
    fn console_getchar() -> usize;
    fn send_ipi(_hartid: usize);
    fn clear_ipi();
    fn shutdown() -> !;
    fn hart_start(_hartid: usize, _start_addr: usize, _opaque: usize);
}

/// memory management arch trait
pub trait ArchMemory {
    type PageTable: ArchPageTable;
    fn tlb_flush();
    fn current_root_ppn() -> usize;
    fn activate(ppn: usize);
    // fn update_pagetable(_bits: usize);
    // fn get_token_by_ppn(_ppn: usize) -> usize;
    // fn current_token() -> usize;
}

pub trait ArchPageTableEntry: Into<usize> + From<usize> + Clone + Copy {
    /// create a new page table entry from ppn and flags
    fn new(ppn: usize, flags: MappingFlags) -> Self;
    /// get the physical page number
    fn ppn(&self) -> usize;
    /// get the pte permission flags
    fn flags(&self) -> MappingFlags;
    /// set flags
    fn set_flags(&mut self, flags: MappingFlags);
    /// clear all data
    fn reset(&mut self);
}

/// trap related arch trait
pub trait ArchTrap {
    type TrapContext: ArchTrapContext;
    fn trap_init();
    fn trap_restore(_cx: &mut <Self as ArchTrap>::TrapContext);
    fn set_kernel_trap_entry();
    fn set_user_trap_entry();
    fn read_epc() -> usize;
    fn read_trap_type() -> TrapType;
}

pub trait ArchTrapContext:
    Index<TrapArgs, Output = usize> + IndexMut<TrapArgs, Output = usize>
{
    fn app_init_cx(entry: usize, sp: usize) -> Self;
    fn update_cx(&mut self, entry: usize, sp: usize, argc: usize, argv: usize, envp: usize);
    fn get_syscall_id(&self) -> usize {
        self[TrapArgs::SYSCALL]
    }
    fn get_syscall_args(&self) -> [usize; 6] {
        [
            self[TrapArgs::A0],
            self[TrapArgs::A1],
            self[TrapArgs::A2],
            self[TrapArgs::A3],
            self[TrapArgs::A4],
            self[TrapArgs::A5],
        ]
    }
}

/// time related arch trait
pub trait ArchTime {
    fn get_time() -> usize;
    fn set_timer(_time_value: u64) -> usize;
}

/// arch info
pub trait ArchInfo {
    const ARCH_NAME: &'static str = "unknown";
}

/// full arch trait
pub trait FullVirtArch: ArchInt + ArchAsm + ArchSbi + ArchTrap + ArchTime + ArchInfo {
    // should impl all traits above
}

use driver::handle_irq;

pub fn ext_int_handler() {
    #[cfg(feature = "async_fs")]
    {
        handle_irq();
    }
    #[cfg(not(feature = "async_fs"))]
    {
        use arch::{Arch, ArchTrap};
        let trap_type = Arch::read_trap_type(None); // scause::read();
        let sepc = Arch::read_epc(); // sepc::read();
        panic!(
            "hart: {}, kernel SupervisorExternal interrupt {:#x?} is unsupported, sepc = {:#x}",
            crate::cpu::get_hartid(),
            trap_type,
            sepc
        )
    }
}

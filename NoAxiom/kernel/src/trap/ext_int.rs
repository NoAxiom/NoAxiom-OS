pub fn ext_int_handler() {
    #[cfg(any(feature = "interruptable_async", feature = "full_func"))]
    {
        driver::handle_irq();
    }
    #[cfg(feature = "async")]
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

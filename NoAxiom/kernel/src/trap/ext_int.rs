//! external interrupt handler

pub fn ext_int_handler() {
    driver::manager::handle_irq();
}

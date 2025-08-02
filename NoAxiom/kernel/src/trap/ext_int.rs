use driver::manager::handle_irq;

pub fn ext_int_handler() {
    handle_irq();
}

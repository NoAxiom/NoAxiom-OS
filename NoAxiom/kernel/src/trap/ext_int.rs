pub fn ext_int_handler() {
    driver::interrupt::handle_irq();
}

mod log;

pub fn log_init() {
    log::log_init();
}

pub fn init(dtb: usize) {
    driver::init(dtb);
}

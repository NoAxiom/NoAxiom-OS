pub mod log;

pub fn driver_init(dtb: usize) {
    driver::init(dtb);
}

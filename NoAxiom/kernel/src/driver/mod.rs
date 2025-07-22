pub mod log;
pub mod base;

pub fn driver_init(dtb: usize) {
    driver::init(dtb);
}

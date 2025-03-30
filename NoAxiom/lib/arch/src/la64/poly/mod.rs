pub mod console;

pub fn la64_dev_init() {
    devices::prepare_drivers();
    if let Some(fdt) = polyhal::common::get_fdt() {
        for node in fdt.all_nodes() {
            devices::try_to_add_device(&node);
        }
    } else {
        log::warn!("No FDT found");
        println!("No FDT found");
    }

    // get devices and init
    devices::regist_devices_irq();

    println!("la64_dev_init done");
}

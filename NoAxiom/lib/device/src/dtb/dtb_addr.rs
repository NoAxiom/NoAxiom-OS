use arch::consts::KERNEL_ADDR_OFFSET;
use ksync::Once;

static DTB_ADDR: Once<usize> = Once::new();

pub fn init_dtb_addr(dtb: usize) {
    DTB_ADDR.call_once(|| dtb | KERNEL_ADDR_OFFSET);
}

pub fn get_dtb_addr() -> Option<usize> {
    DTB_ADDR.get().map(|x| *x)
}

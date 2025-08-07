use arch::{
    consts::{IO_ADDR_OFFSET, KERNEL_ADDR_OFFSET},
    Arch, ArchMemory, ArchTime,
};
use config::mm::PAGE_SIZE;

/*
// for C ffi test
unsafe extern "C" {
    pub fn ahci_mdelay(ms: u32);
    pub fn ahci_printf(fmt: *const u8, _: ...) -> i32;
    pub fn ahci_malloc_align(size: u64, align: u32) -> u64;
    pub fn ahci_sync_dcache();
    pub fn ahci_phys_to_uncached(va: u64) -> u64;
    pub fn ahci_virt_to_phys(va: u64) -> u64;
}
*/

// 等待数毫秒
pub fn ahci_mdelay(ms: u32) {
    let time = Arch::get_time();
    let freq = Arch::get_freq();
    let target = time + (freq * ms as usize) / 1000;
    while Arch::get_time() < target {}
}

// 同步dcache中所有cached和uncached访存请求
pub fn ahci_sync_dcache() {
    Arch::sync_dcache();
}

// 分配按align字节对齐的内存
pub fn ahci_malloc_align(size: u64, align: u32) -> u64 {
    extern crate alloc;
    use alloc::boxed::Box;
    assert!(size < PAGE_SIZE as u64);
    assert!(align.is_power_of_two() && (PAGE_SIZE & (align - 1) as usize) == 0);
    let frame = memory::frame::frame_alloc().unwrap();
    let frame = Box::leak(Box::new(frame));
    let addr = frame.kernel_vpn().as_va_usize() as u64;
    addr
}

// 物理地址转换为uncached虚拟地址
pub fn ahci_phys_to_uncached(pa: u64) -> u64 {
    pa | IO_ADDR_OFFSET as u64
}

// cached虚拟地址转换为物理地址
// ahci dma可以接受64位的物理地址
pub fn ahci_virt_to_phys(va: u64) -> u64 {
    va & !(KERNEL_ADDR_OFFSET as u64) & !(IO_ADDR_OFFSET as u64)
}

use alloc::{boxed::Box, vec::Vec};

use arch::{
    consts::{IO_ADDR_OFFSET, KERNEL_ADDR_OFFSET},
    Arch, ArchMemory,
};
use config::mm::PAGE_SIZE;
use smoltcp::phy::Device;

use crate::net::{ls2k1000_gmac::eth_defs::LsGmacInner, utils::get_time_instant};

/*
// for C test
unsafe extern "C" {
    pub fn eth_printf(fmt: *const u8, _: ...) -> i32;
    pub fn eth_sync_dcache();
    pub fn eth_virt_to_phys(va: u64) -> u32;
    pub fn eth_phys_to_virt(pa: u32) -> u64;
    pub fn eth_malloc_align(size: u64, align: u32) -> u64;
    pub fn eth_handle_tx_buffer(p: u64, buffer: u64) -> u32;
    pub fn eth_handle_rx_buffer(buffer: u64, length: u32) -> u64;
    pub fn eth_rx_ready(gmacdev: *mut net_device);
    pub fn eth_update_linkstate(gmacdev: *mut net_device, status: u32);
    pub fn eth_isr_install();
}
*/

// 同步dcache中所有cached和uncached访存请求
pub fn eth_sync_dcache() {
    Arch::sync_dcache();
}

// cached虚拟地址转换为物理地址
// dma仅接受32位的物理地址
pub fn eth_virt_to_phys(va: u64) -> u32 {
    (va & !(KERNEL_ADDR_OFFSET as u64) & !(IO_ADDR_OFFSET as u64)) as u32
}

// 物理地址转换为cached虚拟地址
pub fn eth_phys_to_virt(pa: u32) -> u64 {
    pa as u64 | KERNEL_ADDR_OFFSET as u64
}

// 物理地址转换为uncached虚拟地址
pub fn eth_phys_to_uncached(pa: u64) -> u64 {
    pa as u64 | IO_ADDR_OFFSET as u64
}

// 分配按align字节对齐的内存
pub fn eth_malloc_align(size: u64, align: u32) -> u64 {
    assert!(size < PAGE_SIZE as u64);
    assert!(align.is_power_of_two() && (PAGE_SIZE & (align - 1) as usize) == 0);
    let frame = memory::frame::frame_alloc().unwrap();
    // SAFETY: there's no memory leak since it's static
    let frame = Box::leak(Box::new(frame));
    let addr = frame.kernel_vpn().as_va_usize() as u64;
    addr
}

// 处理tx buffer
//（OS可能会有自定义格式的存储单元）
// p是OS传递给驱动的存储单元
// buffer是驱动分配的dma内存
// 将p的数据copy到buffer中
// 返回数据总长度
pub fn eth_handle_tx_buffer(p: &[u8], buffer: u64) -> u32 {
    let buffer = unsafe {
        core::slice::from_raw_parts_mut(core::mem::transmute::<u64, *mut u8>(buffer), p.len())
    };
    buffer.copy_from_slice(p);
    0
}

// 处理rx buffer
// buffer是接收到的数据，length是字节数
// OS需要分配内存，memcpy接收到的数据，并将地址返回
pub fn eth_handle_rx_buffer(buffer: u64, length: u32) -> Vec<u8> {
    let buffer = unsafe {
        core::slice::from_raw_parts_mut(
            core::mem::transmute::<u64, *mut u8>(buffer),
            length as usize,
        )
    };
    let mut vec = Vec::with_capacity(length as usize);
    vec.extend_from_slice(buffer);
    vec
}

// 中断isr通知OS可以调用rx函数
pub fn eth_rx_ready(gmacdev: &mut LsGmacInner) {
    gmacdev.receive(get_time_instant());
}

// 中断isr通知链路状态发生变化，status - 1表示up，0表示down
// 链路目前仅支持1000Mbps duplex
pub fn eth_update_linkstate(gmacdev: &mut LsGmacInner, status: u32) {
    log::warn!("Link state updated: {}", status);
}

// OS注册中断，中断号为12
pub fn eth_isr_install() {}
